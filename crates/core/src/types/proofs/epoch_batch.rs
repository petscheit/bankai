//! Epoch Batch Processing Implementation
//!
//! This module handles the batching of epoch updates for efficient processing and verification.
//! It provides functionality to create, manage, and process batches of epoch updates,
//! including merkle tree generation and proof verification.

use crate::{
    cairo_runner::CairoError,
    clients::ClientError,
    db::manager::{DatabaseError, DatabaseManager},
    types::{
        job::JobStatus,
        proofs::epoch_update::{EpochUpdate, ExpectedEpochUpdateOutputs},
        traits::{Exportable, Submittable},
    },
    utils::{
        constants::{SLOTS_PER_EPOCH, TARGET_BATCH_SIZE},
        hashing::get_committee_hash,
        helpers::{self, get_first_slot_for_epoch, slot_to_epoch_id},
        merkle::poseidon::{compute_paths, compute_root, hash_path},
    },
    BankaiClient,
};

use alloy_primitives::FixedBytes;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use starknet::macros::selector;
use starknet_crypto::Felt;
use std::fs;
use thiserror::Error;
use uuid::Uuid;

use std::sync::Arc;
use tracing::{debug, info, trace};

use super::{epoch_update::EpochUpdateError, ProofError};

/// Represents a batch of epoch updates with associated proofs
#[derive(Debug, Serialize, Deserialize)]
pub struct EpochUpdateBatch {
    /// Input data for the batch processing
    pub circuit_inputs: EpochUpdateBatchInputs,
    /// Expected outputs after batch processing
    pub expected_circuit_outputs: ExpectedEpochBatchOutputs,
    /// Merkle paths for verification
    pub merkle_paths: Vec<Vec<Felt>>,
}

/// Input data for batch processing of epoch updates
#[derive(Debug, Serialize, Deserialize)]
pub struct EpochUpdateBatchInputs {
    /// Hash of the committee for this batch
    pub committee_hash: FixedBytes<32>,
    /// List of epoch updates to process
    pub epochs: Vec<EpochUpdate>,
}

/// Expected outputs after batch processing
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExpectedEpochBatchOutputs {
    /// Root hash of the batch merkle tree
    pub batch_root: Felt,
    /// Output data from the latest epoch in the batch
    pub latest_batch_output: ExpectedEpochUpdateOutputs,
}

impl EpochUpdateBatch {
    pub fn name(&self) -> String {
        let first_slot = self
            .circuit_inputs
            .epochs
            .first()
            .unwrap()
            .circuit_inputs
            .header
            .slot;
        let first_epoch = helpers::slot_to_epoch_id(first_slot);
        let last_slot = self
            .circuit_inputs
            .epochs
            .last()
            .unwrap()
            .circuit_inputs
            .header
            .slot;
        let last_epoch = helpers::slot_to_epoch_id(last_slot);
        format!("batch_{}_to_{}", first_epoch, last_epoch)
    }
    /// Creates a new epoch update batch
    ///
    /// Fetches epoch data from the start slot to end slot and creates a batch
    /// with appropriate merkle proofs.
    ///
    /// # Arguments
    /// * `bankai` - Reference to the Bankai client
    ///
    /// # Returns
    /// * `Result<EpochUpdateBatch, EpochBatchError>` - New batch or error
    pub async fn new(bankai: &BankaiClient) -> Result<EpochUpdateBatch, EpochBatchError> {
        let (start_slot, mut end_slot) = bankai
            .starknet_client
            .get_batching_range(&bankai.config)
            .await
            .map_err(|e| EpochBatchError::Client(ClientError::Starknet(e)))?;

        info!("Slots in Term: Start {}, End {}", start_slot, end_slot);
        let epoch_gap = (end_slot - start_slot) / SLOTS_PER_EPOCH;
        info!(
            "Available Epochs in this Sync Committee period: {}",
            epoch_gap
        );

        // if the gap is smaller then x2 the target size, use the entire gap
        if epoch_gap >= TARGET_BATCH_SIZE * 2 {
            end_slot = start_slot + TARGET_BATCH_SIZE * SLOTS_PER_EPOCH;
        }

        info!("Selected Slots: Start {}, End {}", start_slot, end_slot);
        info!("Epoch Count: {}", (end_slot - start_slot) / SLOTS_PER_EPOCH);

        let mut epochs = vec![];

        // Fetch epochs sequentially from start_slot to end_slot, incrementing by 32 each time
        let mut current_slot = start_slot;
        while current_slot < end_slot {
            info!(
                "Getting data for slot: {} Epoch: {} Epochs batch position {}/{}",
                current_slot,
                slot_to_epoch_id(current_slot),
                epochs.len(),
                TARGET_BATCH_SIZE
            );
            let epoch_update = EpochUpdate::new(&bankai.client, current_slot).await?;
            epochs.push(epoch_update);
            current_slot += 32;
        }

        let circuit_inputs = EpochUpdateBatchInputs {
            committee_hash: get_committee_hash(epochs[0].circuit_inputs.aggregate_pub.0),
            epochs,
        };

        let expected_circuit_outputs = ExpectedEpochBatchOutputs::from_inputs(&circuit_inputs);

        let epoch_hashes = circuit_inputs
            .epochs
            .iter()
            .map(|epoch| epoch.expected_circuit_outputs.hash())
            .collect::<Vec<Felt>>();

        let (root, paths) = compute_paths(epoch_hashes.clone());

        // Verify each path matches the root
        for (index, path) in paths.iter().enumerate() {
            let computed_root = hash_path(epoch_hashes[index], path, index);
            if computed_root != root {
                panic!("Path {} does not match root", index);
            }
        }

        let batch = EpochUpdateBatch {
            circuit_inputs,
            expected_circuit_outputs,
            merkle_paths: paths,
        };

        Ok(batch)
    }

    /// Creates a new epoch update batch for a specific epoch range
    ///
    /// # Arguments
    /// * `bankai` - Reference to the Bankai client
    /// * `db_manager` - Reference to the database manager
    /// * `start_epoch` - First epoch in the range
    /// * `end_epoch` - Last epoch in the range
    /// * `job_id` - UUID of the associated job
    ///
    /// # Returns
    /// * `Result<EpochUpdateBatch, EpochBatchError>` - New batch or error
    pub async fn new_by_epoch_range(
        bankai: &BankaiClient,
        db_manager: Arc<DatabaseManager>,
        start_epoch: u64,
        end_epoch: u64,
        job_id: Uuid,
    ) -> Result<EpochUpdateBatch, EpochBatchError> {
        let _permit = bankai
            .config
            .epoch_data_fetching_semaphore
            .clone()
            .acquire_owned()
            .await?;

        let _ = db_manager
            .update_job_status(job_id, JobStatus::StartedFetchingInputs)
            .await;

        let mut epochs = vec![];

        // Fetch epochs sequentially from start_slot to end_slot, incrementing by 32 each time
        let mut current_epoch = start_epoch;
        while current_epoch <= end_epoch {
            let epoch_update =
                EpochUpdate::new(&bankai.client, get_first_slot_for_epoch(current_epoch)).await?;

            epochs.push(epoch_update);
            current_epoch += 1;
        }

        let circuit_inputs = EpochUpdateBatchInputs {
            committee_hash: get_committee_hash(epochs[0].circuit_inputs.aggregate_pub.0),
            epochs,
        };

        let expected_circuit_outputs = ExpectedEpochBatchOutputs::from_inputs(&circuit_inputs);

        let epoch_hashes = circuit_inputs
            .epochs
            .iter()
            .map(|epoch| epoch.expected_circuit_outputs.hash())
            .collect::<Vec<Felt>>();

        let (root, paths) = compute_paths(epoch_hashes.clone());

        // Verify each path matches the root
        current_epoch = start_epoch;
        for (index, path) in paths.iter().enumerate() {
            let computed_root = hash_path(epoch_hashes[index], path, index);
            if computed_root != root {
                panic!("Path {} does not match root", index);
            }
            // Insert merkle paths to database
            //let current_epoch = slot_to_epoch_id(current_slot);
            for (path_index, current_path) in path.iter().enumerate() {
                db_manager
                    .insert_merkle_path_for_epoch(
                        current_epoch,
                        path_index.to_u64().unwrap(),
                        current_path.to_hex_string(),
                    )
                    .await?;
            }
            current_epoch += 1;
        }

        trace!("Paths for epochs {:?}", paths);

        let batch = EpochUpdateBatch {
            circuit_inputs,
            expected_circuit_outputs,
            merkle_paths: paths,
        };

        Ok(batch)
    }
}

impl EpochUpdateBatch {
    /// Loads batch data from a JSON file
    ///
    /// # Arguments
    /// * `first_epoch` - First epoch in the batch
    /// * `last_epoch` - Last epoch in the batch
    ///
    /// # Returns
    /// * `Result<T, EpochBatchError>` - Deserialized data or error
    pub fn from_json<T>(first_epoch: u64, last_epoch: u64) -> Result<T, EpochBatchError>
    where
        T: serde::de::DeserializeOwned,
    {
        info!(
            "Trying to read file batches/epoch_batch/{}_to_{}/input_batch_{}_to_{}.json",
            first_epoch, last_epoch, first_epoch, last_epoch
        );
        // Pattern match for files like: batches/epoch_batch/6709248_to_6710272/input_batch_6709248_to_6710272.json
        let path = format!(
            "batches/epoch_batch/{}_to_{}/input_batch_{}_to_{}.json",
            first_epoch, last_epoch, first_epoch, last_epoch
        );
        debug!(path);
        let glob_pattern = glob::glob(&path)?;

        // Take the first matching file
        let path = glob_pattern.take(1).next().ok_or_else(|| {
            EpochBatchError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No matching file found",
            ))
        })?;

        let json = fs::read_to_string(path.unwrap()).map_err(EpochBatchError::Io)?;
        let json = serde_json::from_str(&json)?;

        Ok(json)
    }
}

impl Exportable for EpochUpdateBatch {
    /// Exports the batch data to a JSON file
    ///
    /// # Returns
    /// * `Result<String, ProofError>` - Path to the exported file or error
    fn export(&self) -> Result<String, ProofError> {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let first_slot = self
            .circuit_inputs
            .epochs
            .first()
            .unwrap()
            .circuit_inputs
            .header
            .slot;
        let first_epoch = helpers::slot_to_epoch_id(first_slot);
        let last_slot = self
            .circuit_inputs
            .epochs
            .last()
            .unwrap()
            .circuit_inputs
            .header
            .slot;
        let last_epoch = helpers::slot_to_epoch_id(last_slot);
        let dir_path = format!("batches/epoch_batch/{}_to_{}", first_epoch, last_epoch);
        fs::create_dir_all(dir_path.clone()).map_err(EpochBatchError::Io)?;
        let path = format!(
            "{}/input_batch_{}_to_{}.json",
            dir_path, first_epoch, last_epoch
        );
        fs::write(path.clone(), json).map_err(EpochBatchError::Io)?;
        Ok(path)
    }
}

impl Submittable<EpochUpdateBatchInputs> for ExpectedEpochBatchOutputs {
    /// Gets the contract selector for batch verification
    ///
    /// # Returns
    /// * `Felt` - Contract selector
    fn get_contract_selector(&self) -> Felt {
        selector!("verify_epoch_batch")
    }

    /// Converts the batch outputs to calldata format
    ///
    /// # Returns
    /// * `Vec<Felt>` - Calldata for contract interaction
    fn to_calldata(&self) -> Vec<Felt> {
        let (header_root_high, header_root_low) = self
            .latest_batch_output
            .beacon_header_root
            .as_slice()
            .split_at(16);
        let (beacon_state_root_high, beacon_state_root_low) = self
            .latest_batch_output
            .beacon_state_root
            .as_slice()
            .split_at(16);
        let (execution_header_hash_high, execution_header_hash_low) = self
            .latest_batch_output
            .execution_header_hash
            .as_slice()
            .split_at(16);
        let (committee_hash_high, committee_hash_low) = self
            .latest_batch_output
            .committee_hash
            .as_slice()
            .split_at(16);
        vec![
            self.batch_root,
            Felt::from_bytes_be_slice(header_root_low),
            Felt::from_bytes_be_slice(header_root_high),
            Felt::from_bytes_be_slice(beacon_state_root_low),
            Felt::from_bytes_be_slice(beacon_state_root_high),
            Felt::from(self.latest_batch_output.slot),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            Felt::from(self.latest_batch_output.n_signers),
            Felt::from_bytes_be_slice(execution_header_hash_low),
            Felt::from_bytes_be_slice(execution_header_hash_high),
            Felt::from(self.latest_batch_output.execution_header_height),
        ]
    }

    /// Creates expected outputs from batch inputs
    ///
    /// # Arguments
    /// * `circuit_inputs` - Batch input data
    ///
    /// # Returns
    /// * `Self` - Expected outputs
    fn from_inputs(circuit_inputs: &EpochUpdateBatchInputs) -> Self {
        let epoch_hashes = circuit_inputs
            .epochs
            .iter()
            .map(|epoch| epoch.expected_circuit_outputs.hash())
            .collect::<Vec<Felt>>();

        let batch_root = compute_root(epoch_hashes.clone());

        let last_epoch_output = circuit_inputs
            .epochs
            .last()
            .unwrap()
            .expected_circuit_outputs
            .clone();

        Self {
            batch_root,
            latest_batch_output: last_epoch_output,
        }
    }
}

/// Possible errors that can occur during epoch batch operations
#[derive(Debug, Error)]
pub enum EpochBatchError {
    /// Error during Cairo program execution
    #[error("Cairo run error: {0}")]
    Cairo(#[from] CairoError),
    /// Database operation error
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    /// Client communication error
    #[error("Client error: {0}")]
    Client(#[from] ClientError),
    /// File system operation error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Resource acquisition error
    #[error("Acquire error: {0}")]
    Acquire(#[from] tokio::sync::AcquireError),
    /// Error processing epoch update
    #[error("Epoch update error: {0}")]
    EpochUpdate(#[from] EpochUpdateError),
    /// JSON serialization/deserialization error
    #[error("Serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
    /// File pattern matching error
    #[error("Pattern error: {0}")]
    Pattern(#[from] glob::PatternError),
}

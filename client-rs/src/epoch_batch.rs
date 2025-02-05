use crate::constants::{SLOTS_PER_EPOCH, TARGET_BATCH_SIZE};
use crate::epoch_update::{EpochUpdate, ExpectedEpochUpdateOutputs};
use crate::helpers::{
    self, calculate_slots_range_for_batch, get_first_slot_for_epoch,
    get_sync_committee_id_by_epoch, slot_to_epoch_id,
};
use crate::traits::{Provable, Submittable};
use crate::utils::hashing::get_committee_hash;

use crate::utils::merkle::poseidon::{compute_paths, compute_root, hash_path};
use crate::{BankaiClient, Error};
use alloy_primitives::FixedBytes;
use hex;
use num_traits::ToPrimitive;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starknet::macros::selector;
use starknet_crypto::Felt;
use std::fs;

use crate::utils::database_manager::DatabaseManager;
use std::sync::Arc;
use tracing::{debug, info, trace};

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochUpdateBatch {
    pub circuit_inputs: EpochUpdateBatchInputs,
    pub expected_circuit_outputs: ExpectedEpochBatchOutputs,
    pub merkle_paths: Vec<Vec<Felt>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochUpdateBatchInputs {
    pub committee_hash: FixedBytes<32>,
    pub epochs: Vec<EpochUpdate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpectedEpochBatchOutputs {
    pub batch_root: Felt,
    pub latest_batch_output: ExpectedEpochUpdateOutputs,
}

impl EpochUpdateBatch {
    pub(crate) async fn new(bankai: &BankaiClient) -> Result<EpochUpdateBatch, Error> {
        let (start_slot, mut end_slot) = bankai
            .starknet_client
            .get_batching_range(&bankai.config)
            .await?;
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
            // Current slot is the starting slot of epoch
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
            //info!("epochspush");
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

    // pub(crate) async fn new_by_slot(
    //     bankai: &BankaiClient,
    //     db_manager: Arc<DatabaseManager>,
    //     slot: u64,
    // ) -> Result<EpochUpdateBatch, Error> {
    //     let _permit = bankai
    //         .config
    //         .epoch_data_fetching_semaphore
    //         .clone()
    //         .acquire_owned()
    //         .await
    //         .map_err(|e| Error::CairoRunError(format!("Semaphore error: {}", e)))?;

    //     let (start_slot, end_slot) = calculate_slots_range_for_batch(slot);
    //     let mut epochs = vec![];

    //     // Fetch epochs sequentially from start_slot to end_slot, incrementing by 32 each time
    //     let mut current_slot = start_slot;
    //     while current_slot < end_slot {
    //         info!(
    //             "Getting data for slot: {} Epoch: {} Epochs batch position {}/{}",
    //             current_slot,
    //             slot_to_epoch_id(current_slot),
    //             epochs.len(),
    //             TARGET_BATCH_SIZE
    //         );
    //         let epoch_update = EpochUpdate::new(&bankai.client, current_slot).await?;

    //         epochs.push(epoch_update);
    //         current_slot += 32;
    //     }

    //     let circuit_inputs = EpochUpdateBatchInputs {
    //         committee_hash: get_committee_hash(epochs[0].circuit_inputs.aggregate_pub.0),
    //         epochs,
    //     };

    //     let expected_circuit_outputs = ExpectedEpochBatchOutputs::from_inputs(&circuit_inputs);

    //     let epoch_hashes = circuit_inputs
    //         .epochs
    //         .iter()
    //         .map(|epoch| epoch.expected_circuit_outputs.hash())
    //         .collect::<Vec<Felt>>();

    //     let (root, paths) = compute_paths(epoch_hashes.clone());

    //     // Verify each path matches the root
    //     current_slot = start_slot;
    //     for (index, path) in paths.iter().enumerate() {
    //         let computed_root = hash_path(epoch_hashes[index], path, index);
    //         if computed_root != root {
    //             panic!("Path {} does not match root", index);
    //         }
    //         // Insert merkle paths to database
    //         let current_epoch = slot_to_epoch_id(current_slot);
    //         for (path_index, current_path) in path.iter().enumerate() {
    //             db_manager
    //                 .insert_merkle_path_for_epoch(
    //                     current_epoch,
    //                     path_index.to_u64().unwrap(),
    //                     current_path.to_hex_string(),
    //                 )
    //                 .await
    //                 .map_err(|e| Error::DatabaseError(e.to_string()))?;
    //         }
    //         current_slot += 32;
    //     }

    //     info!("Paths {:?}", paths);

    //     let batch = EpochUpdateBatch {
    //         circuit_inputs,
    //         expected_circuit_outputs,
    //         merkle_paths: paths,
    //     };

    //     Ok(batch)
    // }

    pub(crate) async fn new_by_epoch_range(
        bankai: &BankaiClient,
        db_manager: Arc<DatabaseManager>,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochUpdateBatch, Error> {
        let _permit = bankai
            .config
            .epoch_data_fetching_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::CairoRunError(format!("Semaphore error: {}", e)))?;

        let mut epochs = vec![];

        // Fetch epochs sequentially from start_slot to end_slot, incrementing by 32 each time
        let calculated_batch_size = end_epoch - start_epoch + 1;
        let mut current_epoch = start_epoch;
        while current_epoch <= end_epoch {
            info!(
                "Getting data for Epoch: {} (SyncCommittee: {}) First slot for this epoch: {} | Epochs batch position {}/{}",
                current_epoch,
                get_sync_committee_id_by_epoch(current_epoch),
                get_first_slot_for_epoch(current_epoch),
                epochs.len()+1,
                calculated_batch_size
            );
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
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;
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
    pub fn from_json<T>(first_epoch: u64, last_epoch: u64) -> Result<T, Error>
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
        let glob_pattern = glob::glob(&path)
            .map_err(|e| Error::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        // Take the first matching file
        let path = glob_pattern.take(1).next().ok_or_else(|| {
            Error::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No matching file found",
            ))
        })?;

        let json = fs::read_to_string(path.unwrap()).map_err(Error::IoError)?;
        serde_json::from_str(&json).map_err(|e| Error::DeserializeError(e.to_string()))
    }
}

impl Provable for EpochUpdateBatch {
    fn id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"epoch_update_batch");
        hasher.update(self.expected_circuit_outputs.batch_root.to_bytes_be());
        hex::encode(hasher.finalize().as_slice())
    }

    fn export(&self) -> Result<String, Error> {
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
        fs::create_dir_all(dir_path.clone()).map_err(Error::IoError)?;
        let path = format!(
            "{}/input_batch_{}_to_{}.json",
            dir_path, first_epoch, last_epoch
        );
        fs::write(path.clone(), json).map_err(Error::IoError)?;
        Ok(path)
    }

    fn proof_type(&self) -> crate::traits::ProofType {
        crate::traits::ProofType::EpochBatch
    }

    fn pie_path(&self) -> String {
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
        format!(
            "batches/epoch_batch/{}_to_{}/pie_batch_{}_to_{}.zip",
            first_epoch, last_epoch, first_epoch, last_epoch
        )
    }

    fn inputs_path(&self) -> String {
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
        format!(
            "batches/epoch_batch/{}_to_{}/input_batch_{}_to_{}.json",
            first_epoch, last_epoch, first_epoch, last_epoch
        )
    }
}

impl Submittable<EpochUpdateBatchInputs> for ExpectedEpochBatchOutputs {
    fn get_contract_selector(&self) -> Felt {
        selector!("verify_epoch_batch")
    }

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

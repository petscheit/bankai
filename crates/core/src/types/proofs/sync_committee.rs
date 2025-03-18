//! Sync Committee Update Processing Implementation
//! 
//! This module handles the verification and processing of sync committee updates on the beacon chain.
//! It provides functionality to create, manage, and verify sync committee transitions, including
//! merkle proof generation and verification for committee membership.

use std::fs;

use crate::{
    clients::beacon_chain::{BeaconError, BeaconRpcClient},
    types::traits::{Exportable, Submittable},
    utils::{hashing::get_committee_hash, helpers, merkle},
};
use alloy_primitives::FixedBytes;
use beacon_state_proof::state_proof_fetcher::{StateProofFetcher, SyncCommitteeProof, TreeHash};
use bls12_381::G1Affine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starknet::{core::types::Felt, macros::selector};
use thiserror::Error;

use super::ProofError;

/// Represents a sync committee update with associated proofs and verification data
#[derive(Debug, Serialize, Deserialize)]
pub struct SyncCommitteeUpdate {
    /// Input data for the circuit verification
    pub circuit_inputs: CommitteeCircuitInputs,
    /// Expected outputs after processing
    pub expected_circuit_outputs: ExpectedCircuitOutputs,
}

impl SyncCommitteeUpdate {
    pub fn name(&self) -> String {
        format!("committee_{}", helpers::get_sync_committee_id_by_slot(self.circuit_inputs.beacon_slot) + 1)
    }
    /// Creates a new sync committee update for a given slot
    ///
    /// # Arguments
    /// * `client` - Reference to the beacon chain client
    /// * `slot` - Slot number to create update for
    ///
    /// # Returns
    /// * `Result<SyncCommitteeUpdate, SyncCommitteeError>` - New update or error
    pub async fn new(
        client: &BeaconRpcClient,
        slot: u64,
    ) -> Result<SyncCommitteeUpdate, SyncCommitteeError> {
        let state_proof_fetcher = StateProofFetcher::new(client.rpc_url.clone());
        let proof = state_proof_fetcher
            .fetch_next_sync_committee_proof(slot)
            .await?;
        let circuit_inputs = CommitteeCircuitInputs::from(proof);
        let mut expected_circuit_outputs = ExpectedCircuitOutputs::from_inputs(&circuit_inputs);
        
        // ToDo: revamp traits to prevent the pot. wrong root from being written
        let header_res = client.get_header(slot).await?;
        let state_root = header_res.data.header.message.state_root;
        expected_circuit_outputs.state_root = state_root;

        Ok(SyncCommitteeUpdate {
            circuit_inputs,
            expected_circuit_outputs,
        })
    }

    /// Loads sync committee update data from a JSON file
    ///
    /// # Arguments
    /// * `slot` - Slot number to load data for
    ///
    /// # Returns
    /// * `Result<T, SyncCommitteeError>` - Deserialized data or error
    pub fn from_json<T>(slot: u64) -> Result<T, SyncCommitteeError>
    where
        T: serde::de::DeserializeOwned,
    {
        let committee_id = helpers::get_sync_committee_id_by_slot(slot) + 1;
        let path = format!("batches/committee/committee_{}/input_{}.json", committee_id, slot);
        println!("path: {}", path);
        let json_string: String = fs::read_to_string(path)?;
        let json = serde_json::from_str(&json_string)?;
        Ok(json)
    }
}

impl Exportable for SyncCommitteeUpdate {
    /// Exports the update data to a JSON file
    ///
    /// # Returns
    /// * `Result<String, ProofError>` - Path to the exported file or error
    fn export(&self) -> Result<String, ProofError> {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let dir_path = format!("batches/committee/{}", self.name());
        let _ = fs::create_dir_all(&dir_path).map_err(SyncCommitteeError::Io)?;

        let path = format!(
            "{}/input_{}.json",
            dir_path, self.circuit_inputs.beacon_slot
        );
        let _ = fs::write(path.clone(), json).map_err(SyncCommitteeError::Io);
        Ok(path)
    }
}

/// Represents a proof for updating the sync committee, containing necessary verification data
/// for validating sync committee transitions in the beacon chain.
#[derive(Debug, Serialize, Deserialize)]
pub struct CommitteeCircuitInputs {
    /// The beacon chain slot number for this proof
    pub beacon_slot: u64,
    /// Merkle branch proving inclusion of the next sync committee
    pub next_sync_committee_branch: Vec<FixedBytes<32>>,
    /// The aggregated public key of the next sync committee
    pub next_aggregate_sync_committee: FixedBytes<48>,
    /// Merkle root of the committee's public keys
    pub committee_keys_root: FixedBytes<32>,
}

impl CommitteeCircuitInputs {
    /// Computes the state root by hashing the committee keys root and the aggregate pubkey.
    ///
    /// # Returns
    /// * `FixedBytes<32>` - The computed state root
    pub fn compute_state_root(&self) -> FixedBytes<32> {
        // Pad the 48-byte aggregate pubkey to 64 bytes for hashing
        let mut padded_aggregate = vec![0u8; 64];
        padded_aggregate[..48].copy_from_slice(&self.next_aggregate_sync_committee[..]);
        let aggregate_root: FixedBytes<32> =
            FixedBytes::from_slice(&Sha256::digest(&padded_aggregate));

        // Prepare leaf data by concatenating the committee keys root and aggregate root
        let mut leaf_data = [0u8; 64];
        leaf_data[0..32].copy_from_slice(self.committee_keys_root.as_slice());
        leaf_data[32..64].copy_from_slice(aggregate_root.as_slice());
        let leaf = FixedBytes::from_slice(&Sha256::digest(leaf_data));

        // Compute the state root using the Merkle path
        merkle::sha256::hash_path(self.next_sync_committee_branch.clone(), leaf, 55)
    }
}

/// Expected outputs after processing a sync committee update
#[derive(Debug, Serialize, Deserialize)]
pub struct ExpectedCircuitOutputs {
    /// The state root containing the new sync committee
    pub state_root: FixedBytes<32>,
    /// The slot containing the state_root
    pub slot: u64,
    /// The hash of the new sync committee
    pub committee_hash: FixedBytes<32>,
}

impl Submittable<CommitteeCircuitInputs> for ExpectedCircuitOutputs {
    /// Creates expected outputs from circuit inputs
    ///
    /// # Arguments
    /// * `circuit_inputs` - Input data for the circuit
    ///
    /// # Returns
    /// * `Self` - Expected outputs after processing
    fn from_inputs(circuit_inputs: &CommitteeCircuitInputs) -> Self {
        let mut compressed_aggregate_pubkey = [0u8; 48];
        compressed_aggregate_pubkey
            .copy_from_slice(circuit_inputs.next_aggregate_sync_committee.as_slice());
        let committee_hash =
            get_committee_hash(G1Affine::from_compressed(&compressed_aggregate_pubkey).unwrap());
        Self {
            state_root: circuit_inputs.compute_state_root(),
            slot: circuit_inputs.beacon_slot,
            committee_hash,
        }
    }

    /// Converts the outputs to calldata format for contract interaction
    ///
    /// # Returns
    /// * `Vec<Felt>` - Calldata as field elements
    fn to_calldata(&self) -> Vec<Felt> {
        let (state_root_high, state_root_low) = self.state_root.as_slice().split_at(16);
        let (committee_hash_high, committee_hash_low) = self.committee_hash.as_slice().split_at(16);
        vec![
            Felt::from_bytes_be_slice(state_root_low),
            Felt::from_bytes_be_slice(state_root_high),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            Felt::from(self.slot),
        ]
    }

    /// Gets the contract selector for committee verification
    ///
    /// # Returns
    /// * `Felt` - Contract selector as a field element
    fn get_contract_selector(&self) -> Felt {
        selector!("verify_committee_update")
    }
}

impl From<SyncCommitteeProof> for CommitteeCircuitInputs {
    /// Converts a `SyncCommitteeProof` into a `CommitteeCircuitInputs`.
    ///
    /// # Arguments
    /// * `committee_proof` - The original sync committee proof to convert
    ///
    /// # Returns
    /// * `Self` - New circuit inputs instance
    fn from(committee_proof: SyncCommitteeProof) -> Self {
        let committee_keys_root = &committee_proof.next_sync_committee.pubkeys.tree_hash_root();

        Self {
            beacon_slot: committee_proof.slot,
            next_sync_committee_branch: committee_proof
                .proof
                .into_iter()
                .map(|bytes| FixedBytes::from_slice(bytes.as_slice()))
                .collect(),
            next_aggregate_sync_committee: FixedBytes::from_slice(
                committee_proof
                    .next_sync_committee
                    .aggregate_pubkey
                    .as_serialized(),
            ),
            committee_keys_root: FixedBytes::from_slice(committee_keys_root.as_slice()),
        }
    }
}

/// Possible errors that can occur during sync committee operations
#[derive(Debug, Error)]
pub enum SyncCommitteeError {
    /// Error communicating with beacon node
    #[error("Beacon error: {0}")]
    Beacon(#[from] BeaconError),
    /// File system operation error
    #[error("Io error: {0}")]
    Io(#[from] std::io::Error),
    /// Error processing beacon state proof
    #[error("Beacon state proof error")]
    BeaconStateProof(beacon_state_proof::error::Error),
    /// JSON serialization/deserialization error
    #[error("Deserialize error: {0}")]
    DeserializeError(#[from] serde_json::Error),
}

impl From<beacon_state_proof::error::Error> for SyncCommitteeError {
    fn from(error: beacon_state_proof::error::Error) -> Self {
        SyncCommitteeError::BeaconStateProof(error)
    }
}

use alloy_primitives::{FixedBytes, U64};
use beacon_state_proof::state_proof_fetcher::{SyncCommitteeProof, TreeHash};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use crate::utils::{hashing::get_committee_hash, merkle};
use bls12_381::{G1Affine, G1Projective, G2Affine};
use alloy_rpc_types_beacon::header::HeaderResponse;
use starknet::core::types::Felt;

/// Represents a proof for updating the sync committee, containing necessary verification data
/// for validating sync committee transitions in the beacon chain.
#[derive(Debug)]
pub struct SyncCommitteeUpdateProof {
    /// The beacon chain slot number for this proof
    pub beacon_slot: U64,
    /// Merkle branch proving inclusion of the next sync committee
    pub next_sync_committee_branch: Vec<FixedBytes<32>>,
    /// The aggregated public key of the next sync committee
    pub next_aggregate_sync_committee: FixedBytes<48>,
    /// Merkle root of the committee's public keys
    pub committee_keys_root: FixedBytes<32>,
}

impl SyncCommitteeUpdateProof {
    /// Computes the state root by hashing the committee keys root and the aggregate pubkey.
    ///
    /// # Returns
    /// 
    /// * `Ok(FixedBytes<32>)` - The computed state root as a 32-byte hash.
    /// * `Err(SyncCommitteeUpdateError)` - If an error occurs during computation.
    pub fn compute_state_root(&self) -> FixedBytes<32> {
        // Pad the 48-byte aggregate pubkey to 64 bytes for hashing
        let mut padded_aggregate = vec![0u8; 64];
        padded_aggregate[..48].copy_from_slice(&self.next_aggregate_sync_committee[..]);
        let aggregate_root: FixedBytes<32> = FixedBytes::from_slice(&Sha256::digest(&padded_aggregate));

        // Prepare leaf data by concatenating the committee keys root and aggregate root
        let mut leaf_data = [0u8; 64];
        leaf_data[0..32].copy_from_slice(self.committee_keys_root.as_slice());
        leaf_data[32..64].copy_from_slice(aggregate_root.as_slice());
        let leaf = FixedBytes::from_slice(&Sha256::digest(&leaf_data));

        // Compute the state root using the Merkle path
        let state_root = merkle::hash_merkle_path(self.next_sync_committee_branch.clone(), leaf, 55);
        state_root
    }
}

impl From<SyncCommitteeProof> for SyncCommitteeUpdateProof {
    /// Converts a `SyncCommitteeProof` into a `SyncCommitteeUpdateProof`.
    ///
    /// # Arguments
    ///
    /// * `committee_proof` - The original sync committee proof to convert.
    ///
    /// # Returns
    ///
    /// A new `SyncCommitteeUpdateProof` instance.
    fn from(committee_proof: SyncCommitteeProof) -> Self {
        let committee_keys_root = &committee_proof.next_sync_committee.pubkeys.tree_hash_root();
        Self {
            beacon_slot: U64::from(committee_proof.slot),
            next_sync_committee_branch: committee_proof.proof.into_iter()
                .map(|bytes| FixedBytes::from_slice(&bytes.as_bytes()))
                .collect(),
            next_aggregate_sync_committee: FixedBytes::from_slice(&committee_proof.next_sync_committee.aggregate_pubkey.as_serialized()),
            committee_keys_root: FixedBytes::from_slice(committee_keys_root.as_bytes()),
        }
    }
}


impl Serialize for SyncCommitteeUpdateProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("SyncCommitteeUpdateProof", 4)?;
        
        state.serialize_field("beacon_slot", &self.beacon_slot)?;
        state.serialize_field("next_sync_committee_branch", &self.next_sync_committee_branch)?;
        state.serialize_field("next_aggregate_sync_committee", &hex::encode(self.next_aggregate_sync_committee))?;
        state.serialize_field("committee_keys_root", &hex::encode(self.committee_keys_root))?;
        
        state.end()
    }
}

/// Represents the public keys of sync committee validators and their aggregate
#[derive(Debug, Clone)]
pub struct SyncCommitteeValidatorPubs {
    /// Individual public keys of all validators in the committee
    pub validator_pubs: Vec<G1Affine>,
    /// Aggregated public key of all validators combined
    pub aggregate_pub: G1Affine,
}

impl SyncCommitteeValidatorPubs {
    // computes the committee hash we use thoughout the project to identify the committee
    pub fn get_committee_hash(&self) -> FixedBytes<32> {
        get_committee_hash(self.aggregate_pub)
    }
}

impl From<Vec<String>> for SyncCommitteeValidatorPubs {
    /// Converts a vector of hex-encoded public key strings into `SyncCommitteeValidatorPubs`.
    ///
    /// # Arguments
    ///
    /// * `validator_pubs` - A vector of hex-encoded public key strings.
    ///
    /// # Returns
    ///
    /// A new `SyncCommitteeValidatorPubs` instance with parsed public keys.
    fn from(validator_pubs: Vec<String>) -> Self {
        let validator_pubs = validator_pubs.iter().map(|s| {
            let mut bytes = [0u8; 48];
            let hex_str = s.trim_start_matches("0x");
            hex::decode_to_slice(hex_str, &mut bytes).unwrap();
            G1Affine::from_compressed(&bytes).unwrap()
        }).collect::<Vec<_>>();

        // Aggregate all public keys into a single G1Projective point
        let aggregate_pub = validator_pubs.iter().fold(G1Projective::identity(), |acc, pubkey| acc.add_mixed(pubkey));
        Self { validator_pubs, aggregate_pub: aggregate_pub.into() }
    }
}

/// Represents a beacon chain block header
#[derive(Debug, Serialize, Deserialize)]
pub struct BeaconHeader {
    /// Slot number of the block
    pub slot: u64,
    /// Index of the block proposer
    pub proposer_index: u64,
    /// Root hash of the parent block
    pub parent_root: FixedBytes<32>,
    /// Root hash of the state
    pub state_root: FixedBytes<32>,
    /// Root hash of the block body
    pub body_root: FixedBytes<32>,
}

/// Contains all necessary inputs for generating and verifying epoch proofs
#[derive(Debug)]
pub struct EpochProofInputs {
    /// The beacon chain block header
    pub header: BeaconHeader,
    /// BLS signature point in G2
    pub signature_point: G2Affine,
    /// Aggregate public key of all validators
    pub aggregate_pub: G1Affine,
    /// Public keys of validators who didn't sign
    pub non_signers: Vec<G1Affine>,
}

impl EpochProofInputs {
    // ToDo: We should compute the header_root here!
    // The issue is that we re-export HashTree from lighthouse -> beacon-state-proofs -> here
    // This means I cant import TreeHashDerive from the crate, applying it to the BeaconHeader type
    // Probably we should import the BeaconHeader type from lighthouse tbh, need to check how to approach this
    pub fn to_calldata(&self, header_root: FixedBytes<32>) -> Vec<Felt> {
        let (header_root_high, header_root_low) = header_root.as_slice().split_at(16);
        let (state_root_high, state_root_low) = self.header.state_root.as_slice().split_at(16);
        let binding = get_committee_hash(self.aggregate_pub);
        let (committee_hash_high, committee_hash_low) = binding.as_slice().split_at(16);
        vec![
            Felt::from_bytes_be_slice(header_root_low),
            Felt::from_bytes_be_slice(header_root_high),
            Felt::from_bytes_be_slice(state_root_low),
            Felt::from_bytes_be_slice(state_root_high),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            Felt::from(512 - self.non_signers.len() as u64),
            Felt::from(self.header.slot),
        ]
    }
}

impl From<HeaderResponse> for BeaconHeader {
    fn from(header: HeaderResponse) -> Self {
        Self { 
            slot: u64::from(header.data.header.message.slot), 
            proposer_index: u64::from(header.data.header.message.proposer_index), 
            parent_root: header.data.header.message.parent_root, 
            state_root: header.data.header.message.state_root,
            body_root: header.data.header.message.body_root 
        }
    }
}

impl Serialize for EpochProofInputs {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("EpochProofInputs", 4)?;
        
        state.serialize_field("header", &self.header)?;

        let uncompressed = self.signature_point.to_uncompressed();
        
        let mut x0 = [0u8; 48];
        let mut x1 = [0u8; 48];
        let mut y0 = [0u8; 48];
        let mut y1 = [0u8; 48];
        
        x1.copy_from_slice(&uncompressed.as_ref()[0..48]);
        x0.copy_from_slice(&uncompressed.as_ref()[48..96]);
        y1.copy_from_slice(&uncompressed.as_ref()[96..144]);
        y0.copy_from_slice(&uncompressed.as_ref()[144..192]);
        
        // Serialize G2Affine signature point directly
        state.serialize_field("signature_point", &serde_json::json!({
            "x0": format!("0x{}", hex::encode(x0)),
            "x1": format!("0x{}", hex::encode(x1)),
            "y0": format!("0x{}", hex::encode(y0)),
            "y1": format!("0x{}", hex::encode(y1))
        }))?;
        let uncompressed = self.aggregate_pub.to_uncompressed();

        let mut x_bytes = [0u8; 48];
        let mut y_bytes = [0u8; 48];
        
        x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);
        
        // Serialize G1Affine aggregate pub directly
        state.serialize_field("committee_pub", &serde_json::json!({
            "x": format!("0x{}", hex::encode(x_bytes)),
            "y": format!("0x{}", hex::encode(y_bytes))
        }))?;

        let non_signers = self.non_signers.iter().map(|p| {
            let uncompressed = p.to_uncompressed();
            let mut x_bytes = [0u8; 48];
            let mut y_bytes = [0u8; 48];
            x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
            y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);
            serde_json::json!({
                "x": format!("0x{}", hex::encode(x_bytes)),
                "y": format!("0x{}", hex::encode(y_bytes))
            })
        }).collect::<Vec<_>>();
        state.serialize_field("non_signers", &non_signers)?;
        
        state.end()
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct ContractInitializationData {
    pub(crate) committee_id: u64,
    pub(crate) committee_hash: FixedBytes<32>,
    pub(crate) committee_update_program_hash: Felt,
    pub(crate) epoch_update_program_hash: Felt,
}

impl ContractInitializationData {
    pub fn to_calldata(&self) -> Vec<Felt> {
        let (committee_high, committee_low) = self.committee_hash.as_slice().split_at(16);
        vec![
            Felt::from(self.committee_id),
            Felt::from_bytes_be_slice(committee_low),
            Felt::from_bytes_be_slice(committee_high),
            self.committee_update_program_hash,
            self.epoch_update_program_hash,
        ]
    }
}
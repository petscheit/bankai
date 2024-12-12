use std::fs;

use crate::{
    traits::{ProofType, Provable, Submittable},
    utils::{hashing::get_committee_hash, rpc::BeaconRpcClient},
    Error,
};
use alloy_primitives::FixedBytes;
use alloy_rpc_types_beacon::{
    events::light_client_finality::SyncAggregate, header::HeaderResponse,
};
use bls12_381::{G1Affine, G1Projective, G2Affine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use starknet::{core::types::Felt, macros::selector};
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

#[derive(Debug, Serialize)]
pub struct EpochUpdate {
    pub circuit_inputs: EpochCircuitInputs,
    pub expected_circuit_outputs: ExpectedCircuitOutputs,
}

impl EpochUpdate {
    pub async fn new(client: &BeaconRpcClient, slot: u64) -> Result<Self, Error> {
        let circuit_inputs = EpochCircuitInputs::generate_epoch_proof(client, slot).await?;
        let expected_circuit_outputs = ExpectedCircuitOutputs::from_inputs(&circuit_inputs);
        Ok(Self {
            circuit_inputs,
            expected_circuit_outputs,
        })
    }
}

impl Provable for EpochUpdate {
    fn id(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"epoch_update");
        hasher.update(self.circuit_inputs.header.tree_hash_root().as_slice());
        hex::encode(hasher.finalize().as_slice())
    }

    fn export(&self) -> Result<String, Error> {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let dir_path = format!("batches/epoch/{}", self.circuit_inputs.header.slot);
        fs::create_dir_all(dir_path.clone()).map_err(|e| Error::IoError(e))?;
        let path = format!("{}/input_{}.json", dir_path, self.circuit_inputs.header.slot);
        fs::write(path.clone(), json).map_err(|e| Error::IoError(e))?;
        Ok(path)
    }

    fn pie_path(&self) -> String {
        format!("batches/epoch/{}/pie_{}.zip", self.circuit_inputs.header.slot, self.circuit_inputs.header.slot)
    }

    fn proof_type(&self) -> ProofType {
        ProofType::Epoch
    }
}

/// Contains all necessary inputs for generating and verifying epoch proofs
#[derive(Debug)]
pub struct EpochCircuitInputs {
    /// The beacon chain block header
    pub header: BeaconHeader,
    /// BLS signature point in G2
    pub signature_point: G2Affine,
    /// Aggregate public key of all validators
    pub aggregate_pub: G1Affine,
    /// Public keys of validators who didn't sign
    pub non_signers: Vec<G1Affine>,
}

/// Represents a beacon chain block header
#[derive(Debug, Serialize, Deserialize, TreeHash)]
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
        let validator_pubs = validator_pubs
            .iter()
            .map(|s| {
                let mut bytes = [0u8; 48];
                let hex_str = s.trim_start_matches("0x");
                hex::decode_to_slice(hex_str, &mut bytes).unwrap();
                G1Affine::from_compressed(&bytes).unwrap()
            })
            .collect::<Vec<_>>();

        // Aggregate all public keys into a single G1Projective point
        let aggregate_pub = validator_pubs
            .iter()
            .fold(G1Projective::identity(), |acc, pubkey| {
                acc.add_mixed(pubkey)
            });
        Self {
            validator_pubs,
            aggregate_pub: aggregate_pub.into(),
        }
    }
}

impl EpochCircuitInputs {
    pub async fn generate_epoch_proof(
        client: &BeaconRpcClient,
        mut slot: u64,
    ) -> Result<EpochCircuitInputs, Error> {
        // First attempt with original slot
        let header = match client.get_header(slot).await {
            Ok(header) => header,
            Err(Error::EmptySlotDetected(_)) => {
                slot += 1;
                println!("Empty slot detected! Fetching slot: {}", slot);
                client.get_header(slot).await?
            }
            Err(e) => return Err(e), // Propagate other errors immediately
        };

        let sync_agg = client.get_sync_aggregate(slot).await?;
        let validator_pubs = client.get_sync_committee_validator_pubs(slot).await?;

        // Process the sync committee data
        let signature_point = Self::extract_signature_point(&sync_agg)?;
        let non_signers = Self::derive_non_signers(&sync_agg, &validator_pubs);

        Ok(EpochCircuitInputs {
            header: header.into(),
            signature_point,
            aggregate_pub: validator_pubs.aggregate_pub,
            non_signers,
        })
    }

    /// Extracts and validates the BLS signature point from the sync aggregate
    fn extract_signature_point(sync_agg: &SyncAggregate) -> Result<G2Affine, Error> {
        let mut bytes = [0u8; 96];
        bytes.copy_from_slice(&sync_agg.sync_committee_signature.0);
        match G2Affine::from_compressed(&bytes).into() {
            Some(point) => Ok(point),
            None => Err(Error::InvalidBLSPoint),
        }
    }

    /// Identifies validators who didn't sign the sync committee message
    /// Returns their public keys as G1Affine points
    fn derive_non_signers(
        sync_aggregate: &SyncAggregate,
        validator_pubs: &SyncCommitteeValidatorPubs,
    ) -> Vec<G1Affine> {
        let bits = Self::convert_bits_to_bool_array(&sync_aggregate.sync_committee_bits);
        validator_pubs
            .validator_pubs
            .iter()
            .enumerate()
            .filter_map(|(i, pubkey)| if !bits[i] { Some(*pubkey) } else { None })
            .collect()
    }

    /// Converts a byte array of participation bits into a boolean array
    /// Each bit represents whether a validator signed (true) or didn't sign (false)
    fn convert_bits_to_bool_array(bits: &[u8]) -> Vec<bool> {
        bits.iter()
            .flat_map(|byte| (0..8).map(move |i| (byte & (1 << i)) != 0))
            .collect()
    }
}

impl From<HeaderResponse> for BeaconHeader {
    fn from(header: HeaderResponse) -> Self {
        Self {
            slot: header.data.header.message.slot,
            proposer_index: header.data.header.message.proposer_index,
            parent_root: header.data.header.message.parent_root,
            state_root: header.data.header.message.state_root,
            body_root: header.data.header.message.body_root,
        }
    }
}

// ToDo: I should wrap the G1Affine and g2Affine type to remove the monstrocity
impl Serialize for EpochCircuitInputs {
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
        state.serialize_field(
            "signature_point",
            &serde_json::json!({
                "x0": format!("0x{}", hex::encode(x0)),
                "x1": format!("0x{}", hex::encode(x1)),
                "y0": format!("0x{}", hex::encode(y0)),
                "y1": format!("0x{}", hex::encode(y1))
            }),
        )?;
        let uncompressed = self.aggregate_pub.to_uncompressed();

        let mut x_bytes = [0u8; 48];
        let mut y_bytes = [0u8; 48];

        x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);

        // Serialize G1Affine aggregate pub directly
        state.serialize_field(
            "committee_pub",
            &serde_json::json!({
                "x": format!("0x{}", hex::encode(x_bytes)),
                "y": format!("0x{}", hex::encode(y_bytes))
            }),
        )?;

        let non_signers = self
            .non_signers
            .iter()
            .map(|p| {
                let uncompressed = p.to_uncompressed();
                let mut x_bytes = [0u8; 48];
                let mut y_bytes = [0u8; 48];
                x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
                y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);
                serde_json::json!({
                    "x": format!("0x{}", hex::encode(x_bytes)),
                    "y": format!("0x{}", hex::encode(y_bytes))
                })
            })
            .collect::<Vec<_>>();
        state.serialize_field("non_signers", &non_signers)?;

        state.end()
    }
}

#[derive(Debug, Serialize)]
pub struct ExpectedCircuitOutputs {
    pub header_root: FixedBytes<32>,
    pub state_root: FixedBytes<32>,
    pub committee_hash: FixedBytes<32>,
    pub n_signers: u64,
    pub slot: u64,
}

impl Submittable<EpochCircuitInputs> for ExpectedCircuitOutputs {
    fn from_inputs(circuit_inputs: &EpochCircuitInputs) -> Self {
        Self {
            header_root: circuit_inputs.header.tree_hash_root(),
            state_root: circuit_inputs.header.state_root,
            committee_hash: get_committee_hash(circuit_inputs.aggregate_pub),
            n_signers: 512 - circuit_inputs.non_signers.len() as u64,
            slot: circuit_inputs.header.slot,
        }
    }

    fn to_calldata(&self) -> Vec<Felt> {
        let (header_root_high, header_root_low) = self.header_root.as_slice().split_at(16);
        let (state_root_high, state_root_low) = self.state_root.as_slice().split_at(16);
        let (committee_hash_high, committee_hash_low) = self.committee_hash.as_slice().split_at(16);
        vec![
            Felt::from_bytes_be_slice(header_root_low),
            Felt::from_bytes_be_slice(header_root_high),
            Felt::from_bytes_be_slice(state_root_low),
            Felt::from_bytes_be_slice(state_root_high),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            Felt::from(self.n_signers),
            Felt::from(self.slot),
        ]
    }

    fn get_contract_selector(&self) -> Felt {
        selector!("verify_epoch_update")
    }
}

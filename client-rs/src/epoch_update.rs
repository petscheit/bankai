use std::fs;

use crate::constants;
use crate::{
    execution_header::ExecutionHeaderProof,
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
use starknet_crypto::poseidon_hash_many;
use tracing::info;
use tree_hash::TreeHash;
use tree_hash_derive::TreeHash;

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochUpdate {
    pub circuit_inputs: EpochCircuitInputs,
    pub expected_circuit_outputs: ExpectedEpochUpdateOutputs,
}

impl EpochUpdate {
    pub(crate) async fn new(client: &BeaconRpcClient, slot: u64) -> Result<Self, Error> {
        let circuit_inputs = EpochCircuitInputs::generate_epoch_proof(client, slot).await?;
        let expected_circuit_outputs = ExpectedEpochUpdateOutputs::from_inputs(&circuit_inputs);
        Ok(Self {
            circuit_inputs,
            expected_circuit_outputs,
        })
    }
}

impl EpochUpdate {
    pub fn from_json<T>(slot: u64) -> Result<T, Error>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = format!("batches/epoch/{}/input_{}.json", slot, slot);
        let json = fs::read_to_string(path).map_err(Error::IoError)?;
        serde_json::from_str(&json).map_err(|e| Error::DeserializeError(e.to_string()))
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
        fs::create_dir_all(dir_path.clone()).map_err(Error::IoError)?;
        let path = format!(
            "{}/input_{}.json",
            dir_path, self.circuit_inputs.header.slot
        );
        fs::write(path.clone(), json).map_err(Error::IoError)?;
        Ok(path)
    }

    fn pie_path(&self) -> String {
        format!(
            "batches/epoch/{}/pie_{}.zip",
            self.circuit_inputs.header.slot, self.circuit_inputs.header.slot
        )
    }

    fn proof_type(&self) -> ProofType {
        ProofType::Epoch
    }

    fn inputs_path(&self) -> String {
        format!(
            "batches/epoch/{}/input_{}.json",
            self.circuit_inputs.header.slot, self.circuit_inputs.header.slot
        )
    }
}

/// Contains all necessary inputs for generating and verifying epoch proofs
#[derive(Debug, Serialize, Deserialize)]
pub struct EpochCircuitInputs {
    /// The beacon chain block header
    pub header: BeaconHeader,
    /// BLS signature point in G2
    pub signature_point: G2Point,
    /// Aggregate public key of all validators
    #[serde(rename = "committee_pub")]
    pub aggregate_pub: G1Point,
    /// Public keys of validators who didn't sign
    pub non_signers: Vec<G1Point>,
    /// Proof of inclusion for the execution payload header
    pub execution_header_proof: ExecutionHeaderProof,
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
    pub(crate) async fn generate_epoch_proof(
        client: &BeaconRpcClient,
        mut slot: u64,
    ) -> Result<EpochCircuitInputs, Error> {
        let mut attempts = 0;

        let header = loop {
            match client.get_header(slot).await {
                Ok(header) => break header,
                Err(Error::EmptySlotDetected(_)) => {
                    attempts += 1;
                    if attempts >= constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS {
                        return Err(Error::EmptySlotDetected(slot));
                    }
                    slot += 1;
                    info!(
                        "Empty slot detected! Attempt {}/{}. Fetching slot: {}",
                        attempts,
                        constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS,
                        slot
                    );
                }
                Err(e) => return Err(e), // Propagate other errors immediately
            }
        };

        let sync_agg = client.get_sync_aggregate(slot).await?;
        let validator_pubs = client.get_sync_committee_validator_pubs(slot).await?;
        // Process the sync committee data
        let signature_point = Self::extract_signature_point(&sync_agg)?;
        let non_signers = Self::derive_non_signers(&sync_agg, &validator_pubs);

        Ok(EpochCircuitInputs {
            header: header.into(),
            signature_point,
            aggregate_pub: G1Point(validator_pubs.aggregate_pub),
            non_signers: non_signers.iter().map(|p| G1Point(*p)).collect(),
            execution_header_proof: ExecutionHeaderProof::fetch_proof(client, slot).await?,
        })
    }

    /// Extracts and validates the BLS signature point from the sync aggregate
    fn extract_signature_point(sync_agg: &SyncAggregate) -> Result<G2Point, Error> {
        let mut bytes = [0u8; 96];
        bytes.copy_from_slice(&sync_agg.sync_committee_signature.0);
        match G2Affine::from_compressed(&bytes).into() {
            Some(point) => Ok(G2Point(point)),
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

#[derive(Debug)]
pub struct G1Point(pub G1Affine);
#[derive(Debug)]
pub struct G2Point(pub G2Affine);

impl Serialize for G1Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let uncompressed = self.0.to_uncompressed();
        let mut x_bytes = [0u8; 48];
        let mut y_bytes = [0u8; 48];

        x_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);

        serde_json::json!({
            "x": format!("0x{}", hex::encode(x_bytes)),
            "y": format!("0x{}", hex::encode(y_bytes))
        })
        .serialize(serializer)
    }
}

impl Serialize for G2Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let uncompressed = self.0.to_uncompressed();
        let mut x0_bytes = [0u8; 48];
        let mut x1_bytes = [0u8; 48];
        let mut y0_bytes = [0u8; 48];
        let mut y1_bytes = [0u8; 48];
        x0_bytes.copy_from_slice(&uncompressed.as_ref()[48..96]);
        x1_bytes.copy_from_slice(&uncompressed.as_ref()[0..48]);
        y0_bytes.copy_from_slice(&uncompressed.as_ref()[144..192]);
        y1_bytes.copy_from_slice(&uncompressed.as_ref()[96..144]);
        serde_json::json!({
            "x0": format!("0x{}", hex::encode(x0_bytes)),
            "x1": format!("0x{}", hex::encode(x1_bytes)),
            "y0": format!("0x{}", hex::encode(y0_bytes)),
            "y1": format!("0x{}", hex::encode(y1_bytes))
        })
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for G1Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into a Value first
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        // Extract x and y coordinates
        let x_str = value["x"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x coordinate"))?;
        let y_str = value["y"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y coordinate"))?;

        // Safely remove "0x" prefix if it exists
        let x_hex = x_str.strip_prefix("0x").unwrap_or(x_str);
        let y_hex = y_str.strip_prefix("0x").unwrap_or(y_str);

        let x_bytes = hex::decode(x_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x hex: {}", e)))?;
        let y_bytes = hex::decode(y_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y hex: {}", e)))?;

        // Combine into uncompressed format
        let mut uncompressed = [0u8; 96];
        uncompressed[0..48].copy_from_slice(&x_bytes);
        uncompressed[48..96].copy_from_slice(&y_bytes);

        // Convert to G1Affine point
        let point = G1Affine::from_uncompressed(&uncompressed).unwrap();

        Ok(G1Point(point))
    }
}

impl<'de> Deserialize<'de> for G2Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize into a Value first
        let value: serde_json::Value = serde_json::Value::deserialize(deserializer)?;

        // Extract coordinates
        let x0_str = value["x0"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x0 coordinate"))?;
        let x1_str = value["x1"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing x1 coordinate"))?;
        let y0_str = value["y0"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y0 coordinate"))?;
        let y1_str = value["y1"]
            .as_str()
            .ok_or_else(|| serde::de::Error::custom("missing y1 coordinate"))?;

        // Safely remove "0x" prefix if it exists
        let x0_hex = x0_str.strip_prefix("0x").unwrap_or(x0_str);
        let x1_hex = x1_str.strip_prefix("0x").unwrap_or(x1_str);
        let y0_hex = y0_str.strip_prefix("0x").unwrap_or(y0_str);
        let y1_hex = y1_str.strip_prefix("0x").unwrap_or(y1_str);

        // Decode hex strings to bytes
        let x0_bytes = hex::decode(x0_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x0 hex: {}", e)))?;
        let x1_bytes = hex::decode(x1_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid x1 hex: {}", e)))?;
        let y0_bytes = hex::decode(y0_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y0 hex: {}", e)))?;
        let y1_bytes = hex::decode(y1_hex)
            .map_err(|e| serde::de::Error::custom(format!("invalid y1 hex: {}", e)))?;

        // Combine into uncompressed format
        let mut uncompressed = [0u8; 192];
        uncompressed[0..48].copy_from_slice(&x1_bytes);
        uncompressed[48..96].copy_from_slice(&x0_bytes);
        uncompressed[96..144].copy_from_slice(&y1_bytes);
        uncompressed[144..192].copy_from_slice(&y0_bytes);

        // Convert to G2Affine point
        let point = G2Affine::from_uncompressed(&uncompressed).unwrap();

        Ok(G2Point(point))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExpectedEpochUpdateOutputs {
    pub beacon_header_root: FixedBytes<32>,
    pub beacon_state_root: FixedBytes<32>,
    pub slot: u64,
    pub committee_hash: FixedBytes<32>,
    pub n_signers: u64,
    pub execution_header_hash: FixedBytes<32>,
    pub execution_header_height: u64,
}

impl ExpectedEpochUpdateOutputs {
    pub fn hash(&self) -> Felt {
        let felts = self.to_calldata();
        poseidon_hash_many(&felts)
    }
}

impl Submittable<EpochCircuitInputs> for ExpectedEpochUpdateOutputs {
    fn from_inputs(circuit_inputs: &EpochCircuitInputs) -> Self {
        let block_hash: FixedBytes<32> = FixedBytes::from_slice(
            circuit_inputs
                .execution_header_proof
                .execution_payload_header
                .block_hash()
                .into_root()
                .as_bytes(),
        );
        Self {
            beacon_header_root: circuit_inputs.header.tree_hash_root(),
            beacon_state_root: circuit_inputs.header.state_root,
            slot: circuit_inputs.header.slot,
            committee_hash: get_committee_hash(circuit_inputs.aggregate_pub.0),
            n_signers: 512 - circuit_inputs.non_signers.len() as u64,
            execution_header_hash: block_hash,
            execution_header_height: circuit_inputs
                .execution_header_proof
                .execution_payload_header
                .block_number(),
        }
    }

    fn to_calldata(&self) -> Vec<Felt> {
        let (header_root_high, header_root_low) = self.beacon_header_root.as_slice().split_at(16);
        let (beacon_state_root_high, beacon_state_root_low) =
            self.beacon_state_root.as_slice().split_at(16);
        let (execution_header_hash_high, execution_header_hash_low) =
            self.execution_header_hash.as_slice().split_at(16);
        let (committee_hash_high, committee_hash_low) = self.committee_hash.as_slice().split_at(16);
        vec![
            Felt::from_bytes_be_slice(header_root_low),
            Felt::from_bytes_be_slice(header_root_high),
            Felt::from_bytes_be_slice(beacon_state_root_low),
            Felt::from_bytes_be_slice(beacon_state_root_high),
            Felt::from(self.slot),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            Felt::from(self.n_signers),
            Felt::from_bytes_be_slice(execution_header_hash_low),
            Felt::from_bytes_be_slice(execution_header_hash_high),
            Felt::from(self.execution_header_height),
        ]
    }

    fn get_contract_selector(&self) -> Felt {
        selector!("verify_epoch_update")
    }
}

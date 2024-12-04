use alloy_primitives::{FixedBytes, U64};
use beacon_state_proof::state_proof_fetcher::{SyncCommitteeProof, TreeHash};
use sha2::{Sha256, Digest};
use crate::sync_committee::SyncCommitteeUpdateError;
use serde::{Serialize, Deserialize};
use crate::utils::merkle;
use bls12_381::{G1Affine, G1Projective, G2Affine};
use alloy_rpc_types_beacon::header::HeaderResponse;


#[derive(Debug)]
pub struct SyncCommitteeUpdateProof {
    /// Slot of the proof
    pub beacon_slot: U64,
    /// Branch of the proof
    pub next_sync_committee_branch: Vec<FixedBytes<32>>,
    /// The aggregated next sync committee pubkey
    pub next_aggregate_sync_committee: FixedBytes<48>,
    /// The SyncCommittee container contains a list of pubkeys, and the aggregate pubkey
    pub committee_keys_root: FixedBytes<32>,
}

impl SyncCommitteeUpdateProof {
    pub fn compute_state_root(&self) -> Result<FixedBytes<32>, SyncCommitteeUpdateError> {

        // Since the keys are 48 bytes, we pad them to 64 bytes and hash them for the root
        let mut padded_aggregate = vec![0u8; 64];
        padded_aggregate[..48].copy_from_slice(&self.next_aggregate_sync_committee[..]);
        let aggregate_root: FixedBytes<32> = FixedBytes::from_slice(&Sha256::digest(&padded_aggregate));

        let mut leaf_data = [0u8; 64];
        leaf_data[0..32].copy_from_slice(self.committee_keys_root.as_slice());
        leaf_data[32..64].copy_from_slice(aggregate_root.as_slice());
        let leaf = FixedBytes::from_slice(&Sha256::digest(&leaf_data));
        let state_root = merkle::hash_merkle_path(self.next_sync_committee_branch.clone(), leaf, 55);
        Ok(state_root)
    }
}

impl From<SyncCommitteeProof> for SyncCommitteeUpdateProof {
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

#[derive(Debug, Clone)]
pub struct SyncCommitteeValidatorPubs {
    pub validator_pubs: Vec<G1Affine>,
    pub aggregate_pub: G1Affine,
}

impl From<Vec<String>> for SyncCommitteeValidatorPubs {
    fn from(validator_pubs: Vec<String>) -> Self {
        let validator_pubs = validator_pubs.iter().map(|s| {
            let mut bytes = [0u8; 48];
            let hex_str = s.trim_start_matches("0x");
            hex::decode_to_slice(hex_str, &mut bytes).unwrap();
            G1Affine::from_compressed(&bytes).unwrap()
        }).collect::<Vec<_>>();

        let aggregate_pub = validator_pubs.iter().fold(G1Projective::identity(), |acc, pubkey| acc.add_mixed(pubkey));
        Self { validator_pubs, aggregate_pub: aggregate_pub.into() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BeaconHeader {
    pub slot: U64,
    pub proposer_index: U64,
    pub parent_root: FixedBytes<32>,
    pub state_root: FixedBytes<32>,
    pub body_root: FixedBytes<32>,
}

#[derive(Debug)]
pub struct EpochProofInputs {
    pub header: BeaconHeader,
    pub signature_point: G2Affine,
    pub aggregate_pub: G1Affine,
    pub non_signers: Vec<G1Affine>,
}

impl From<HeaderResponse> for BeaconHeader {
    fn from(header: HeaderResponse) -> Self {
        Self { 
            slot: U64::from(header.data.header.message.slot), 
            proposer_index: U64::from(header.data.header.message.proposer_index), 
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
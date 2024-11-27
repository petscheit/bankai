use alloy_primitives::{FixedBytes, U64};
use beacon_state_proof::state_proof_fetcher::{SyncCommitteeProof, TreeHash};
use sha2::{Sha256, Digest};
use crate::sync_committee::SyncCommitteeUpdateError;
use serde::Serialize;
use crate::utils::merkle;

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
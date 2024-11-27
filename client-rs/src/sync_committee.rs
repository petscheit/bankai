use crate::types::SyncCommitteeUpdateProof;
use beacon_state_proof::error::Error as BeaconStateProofError;
use beacon_state_proof::state_proof_fetcher::StateProofFetcher;
pub struct SyncCommitteeUpdate {}

impl SyncCommitteeUpdate {
    pub async fn generate_proof(
        rpc_url: &String,
        slot: u64,
    ) -> Result<SyncCommitteeUpdateProof, SyncCommitteeUpdateError> {

        let state_proof_fetcher = StateProofFetcher::new(rpc_url.clone());
        let proof = state_proof_fetcher
            .fetch_next_sync_committee_proof(slot)
            .await
            .map_err(SyncCommitteeUpdateError::BeaconStateError)?;

        Ok(SyncCommitteeUpdateProof::from(proof))
    }
}

#[derive(Debug)]
pub enum SyncCommitteeUpdateError {
    NoCheckpointSlotRequested,
    BeaconStateError(BeaconStateProofError),
}

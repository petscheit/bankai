use crate::types::{SyncCommitteeUpdateProof, BeaconHeader};
use crate::utils::rpc::BeaconRpcClient;
use beacon_state_proof::state_proof_fetcher::StateProofFetcher;
use crate::Error;

pub struct SyncCommitteeUpdate {}

impl SyncCommitteeUpdate {
    pub async fn generate_proof(
        client: &BeaconRpcClient,
        slot: u64,
    ) -> Result<SyncCommitteeUpdateProof, Error> {

        let state_proof_fetcher = StateProofFetcher::new(client.rpc_url.clone());
        let proof = state_proof_fetcher
            .fetch_next_sync_committee_proof(slot)
            .await
            .map_err(|_| Error::FailedFetchingBeaconState)?;

        let proof = SyncCommitteeUpdateProof::from(proof);

        // Ensure the proof yields the expected state root
        let state_root = proof.compute_state_root();
        let header: BeaconHeader = client.get_header(slot).await?.into();
        if format!("0x{}", hex::encode(state_root)) != header.state_root.to_string() {
            return Err(Error::InvalidProof);
        }

        Ok(SyncCommitteeUpdateProof::from(proof))
    }
}

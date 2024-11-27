mod sync_committee;
mod types;
mod utils;

use sync_committee::{SyncCommitteeUpdate, SyncCommitteeUpdateError};
use types::SyncCommitteeUpdateProof;
use utils::rpc::get_state_root;

#[derive(Debug)]
pub enum Error {
    SyncCommittee(SyncCommitteeUpdateError),
    InvalidProof,
    RpcError(reqwest::Error),
}

struct BankaiClient {
    rpc_url: String,
}

impl BankaiClient {
    pub fn new(rpc_url: String) -> Self {
        Self { rpc_url }
    }

    pub async fn get_sync_committee_update(&self, slot: u64) -> Result<(), Error> {
        let proof: SyncCommitteeUpdateProof =
            SyncCommitteeUpdate::generate_proof(&self.rpc_url, slot)
                .await
                .map_err(Error::SyncCommittee)?;

        let state_root = proof.compute_state_root().map_err(Error::SyncCommittee)?;
        let rpc_state_root = get_state_root(slot, &self.rpc_url)
            .await
            .map_err(Error::RpcError)?;
        if format!("0x{}", hex::encode(state_root)) != rpc_state_root {
            return Err(Error::InvalidProof);
        }

        let json = serde_json::to_string_pretty(&proof).unwrap();
        println!("SyncCommitteeUpdateProof: {}", json);

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client = BankaiClient::new("http://127.0.0.1:5052".to_string());
    client.get_sync_committee_update(6400930).await?;
    Ok(())
}

// MISSED CHECKPOINT SLOT: 6400932

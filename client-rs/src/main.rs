mod sync_committee;
mod types;
mod utils;
mod epoch_update;

use epoch_update::generate_epoch_proof;
use sync_committee::{SyncCommitteeUpdate, SyncCommitteeUpdateError};
use types::SyncCommitteeUpdateProof;
use utils::rpc::BeaconRpcClient;
// use rand::Rng;
// use std::fs::File;
// use std::io::Write;

#[derive(Debug)]
pub enum Error {
    SyncCommittee(SyncCommitteeUpdateError),
    InvalidProof,
    RpcError(reqwest::Error),
    DeserializeError(serde_json::Error),
    FetchSyncCommitteeError
}

struct BankaiClient {
    client: BeaconRpcClient,
}

impl BankaiClient {
    pub fn new(rpc_url: String) -> Self {
        Self { client: BeaconRpcClient::new(rpc_url) }
    }

    pub async fn get_sync_committee_update(&self, slot: u64) -> Result<SyncCommitteeUpdateProof, Error> {
        let proof: SyncCommitteeUpdateProof =
            SyncCommitteeUpdate::generate_proof(&self.client.rpc_url, slot)
                .await
                .map_err(Error::SyncCommittee)?;

        let state_root = proof.compute_state_root().map_err(Error::SyncCommittee)?;
        let rpc_state_root = self.client.get_header(slot).await?;
        if format!("0x{}", hex::encode(state_root)) != rpc_state_root.data.root.to_string() {
            return Err(Error::InvalidProof);
        }

        Ok(proof)
    }

    pub async fn get_epoch_proof(&self, slot: u64) -> Result<(), Error> {
        let header = self.client.get_header(slot).await?;
        let sync_agg = self.client.get_sync_aggregate(slot).await?;
        let validator_pubs = self.client.get_sync_committee_validator_pubs(slot).await?;
        let epoch_proof = generate_epoch_proof(header.into(), sync_agg, validator_pubs)?;
        // Convert to JSON and print
        let json = serde_json::to_string_pretty(&epoch_proof)
            .map_err(Error::DeserializeError)?;
        println!("{}", json);
        Ok(())
    }
    
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let bankai = BankaiClient::new("https://side-radial-morning.ethereum-sepolia.quiknode.pro/006c5ea080a9f60afbb3cc1eb8cc7ab486c9d128".to_string());

    bankai.get_epoch_proof(5808224).await?;
    Ok(())
}

// #[tokio::main]
// async fn main() -> Result<(), Error> {
//     let bankai = BankaiClient::new("https://side-radial-morning.ethereum-sepolia.quiknode.pro/006c5ea080a9f60afbb3cc1eb8cc7ab486c9d128".to_string());
    
//     let num_samples = 47; // Change this to desired number of samples
//     let mut rng = rand::thread_rng();
//     let mut proofs = Vec::new();
    
//     // Generate random slots between 5800064 and 6400932
//     for _ in 0..num_samples {
//         let random_slot = rng.gen_range(5000000..=6400932);
//         match bankai.get_sync_committee_update(random_slot).await {
//             Ok(proof) => {
//                 let json = serde_json::to_string_pretty(&proof).unwrap();
//                 let state_root = bankai.client.get_header(random_slot).await?.data.root.to_string();
//                 let mut json_value: serde_json::Value = serde_json::from_str(&json).unwrap();
//                 if let serde_json::Value::Object(ref mut map) = json_value {
//                     map.insert("expected_state_root".to_string(), serde_json::Value::String(state_root));
//                 }
//                 let json = serde_json::to_string_pretty(&json_value).unwrap();
//                 println!("Generated proof for slot {}", random_slot);
//                 let filename = format!("output/committee_update_{}.json", random_slot);
//                 let mut file = File::create(filename).unwrap();
//                 file.write_all(json.as_bytes()).unwrap();
//                 proofs.push(proof);
//             },
//             Err(e) => println!("Error generating proof for slot {}: {:?}", random_slot, e),
//         }
//     }

//     println!("Generated {} fixtures", proofs.len());
//     Ok(())
// }

// MISSED CHECKPOINT SLOT: 6400932

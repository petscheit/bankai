mod sync_committee;
mod types;
mod utils;
mod epoch_update;

use epoch_update::EpochUpdate;
use sync_committee::SyncCommitteeUpdate;
use types::{SyncCommitteeUpdateProof, EpochProofInputs};
use utils::rpc::BeaconRpcClient;
// use rand::Rng;
// use std::fs::File;
// use std::io::Write;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Debug)]
pub enum Error {
    InvalidProof,
    RpcError(reqwest::Error),
    DeserializeError(serde_json::Error),
    FetchSyncCommitteeError,
    FailedFetchingBeaconState,
    InvalidBLSPoint,
    MissingRpcUrl,
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
            SyncCommitteeUpdate::generate_proof(&self.client, slot)
                .await?;

        Ok(proof)
    }

    pub async fn get_epoch_proof(&self, slot: u64) -> Result<EpochProofInputs, Error> {
        let epoch_proof = EpochUpdate::generate_epoch_proof(&self.client, slot).await?;
        Ok(epoch_proof)
    }
    
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a sync committee update proof for a given slot
    CommitteeUpdate {
        #[arg(long,short)]
        slot: u64,
    },
    /// Generate an epoch update proof for a given slot
    EpochUpdate {
        #[arg(long,short)]
        slot: u64,
    },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional RPC URL (defaults to RPC_URL_BEACON environment variable)
    #[arg(long)]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    
    let rpc_url = cli.rpc_url.or_else(|| env::var("RPC_URL_BEACON").ok())
        .ok_or(Error::MissingRpcUrl)?;
    
    let bankai = BankaiClient::new(rpc_url);

    match cli.command {
        Commands::CommitteeUpdate { slot } => {
            println!("SyncCommittee command received with slot: {}", slot);
            let proof = bankai.get_sync_committee_update(slot).await?;
            let json = serde_json::to_string_pretty(&proof)
                .map_err(Error::DeserializeError)?;
            println!("{}", json);
        }
        Commands::EpochUpdate { slot } => {
            println!("Epoch command received with slot: {}", slot);
            let proof = bankai.get_epoch_proof(slot).await?;
            let json = serde_json::to_string_pretty(&proof)
                .map_err(Error::DeserializeError)?;
            println!("{}", json);
        }
    }

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

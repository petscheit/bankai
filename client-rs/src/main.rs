mod contract_init;
mod epoch_update;
mod sync_committee;
mod traits;
mod utils;

use contract_init::ContractInitializationData;
use epoch_update::EpochUpdate;
use starknet::core::types::Felt;
use sync_committee::SyncCommitteeUpdate;
use utils::{
    rpc::BeaconRpcClient,
    starknet_client::{StarknetClient, StarknetError},
};
// use rand::Rng;
// use std::fs::File;
// use std::io::Write;
use clap::{Parser, Subcommand};
use std::env;

#[derive(Debug)]
pub enum Error {
    InvalidProof,
    RpcError(reqwest::Error),
    DeserializeError(String),
    IoError(std::io::Error),
    StarknetError(StarknetError),
    BlockNotFound,
    FetchSyncCommitteeError,
    FailedFetchingBeaconState,
    InvalidBLSPoint,
    MissingRpcUrl,
    EmptySlotDetected(u64),
}

impl From<StarknetError> for Error {
    fn from(e: StarknetError) -> Self {
        Error::StarknetError(e)
    }
}

struct BankaiConfig {
    contract_class_hash: Felt,
    contract_address: Felt,
    committee_update_program_hash: Felt,
    epoch_update_program_hash: Felt,
    contract_path: String,
}

impl Default for BankaiConfig {
    fn default() -> Self {
        Self {
            contract_class_hash: Felt::from_hex(
                "0x05e54a08e87b40c3f3ec9fd9e10900e61a9d1a08f3b4d0d110b232c43a3661df",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0xf8c47855513a0ac18b1af7b0c61fb45239376b58f06c838f090ad13727220",
            )
            .unwrap(),
            committee_update_program_hash: Felt::from_hex(
                "0x229e5ad2e3b8c6dd4d0319cdd957bbd7bdf2ea685e172b049c3e5f55b0352c1",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x4b5e6a385a98eef562265f5c4769794cee3fecaaaefb47200d8d804c35c20d6",
            )
            .unwrap(),
            contract_path: "../contract/target/dev/bankai_BankaiContract.contract_class.json"
                .to_string(),
        }
    }
}

struct BankaiClient {
    client: BeaconRpcClient,
    starknet_client: StarknetClient,
    config: BankaiConfig,
}

impl BankaiClient {
    pub async fn new(rpc_url: String) -> Self {
        Self {
            client: BeaconRpcClient::new(rpc_url),
            starknet_client: StarknetClient::new("http://127.0.0.1:5050").await.unwrap(),
            config: BankaiConfig::default(),
        }
    }

    pub async fn get_sync_committee_update(
        &self,
        mut slot: u64,
    ) -> Result<SyncCommitteeUpdate, Error> {
        // Before we start generating the proof, we ensure the slot was not missed
        match self.client.get_header(slot).await {
            Ok(header) => header,
            Err(Error::EmptySlotDetected(_)) => {
                slot += 1;
                println!("Empty slot detected! Fetching slot: {}", slot);
                self.client.get_header(slot).await?
            }
            Err(e) => return Err(e), // Propagate other errors immediately
        };

        let proof: SyncCommitteeUpdate = SyncCommitteeUpdate::new(&self.client, slot).await?;

        Ok(proof)
    }

    pub async fn get_epoch_proof(&self, slot: u64) -> Result<EpochUpdate, Error> {
        let epoch_proof = EpochUpdate::new(&self.client, slot).await?;
        Ok(epoch_proof)
    }

    pub async fn get_contract_initialization_data(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<ContractInitializationData, Error> {
        let contract_init = ContractInitializationData::new(&self.client, slot, config).await?;
        Ok(contract_init)
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a sync committee update proof for a given slot
    CommitteeUpdate {
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    /// Generate an epoch update proof for a given slot
    EpochUpdate {
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    /// Generate contract initialization data for a given slot
    ContractInit {
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    DeployContract {
        #[arg(long, short)]
        slot: u64,
    },
    SubmitEpochProof {
        #[arg(long, short)]
        slot: u64,
    },
    SubmitNextCommittee,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional RPC URL (defaults to RPC_URL_BEACON environment variable)
    #[arg(long, short)]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    let rpc_url = cli
        .rpc_url
        .or_else(|| env::var("RPC_URL_BEACON").ok())
        .ok_or(Error::MissingRpcUrl)?;

    let bankai = BankaiClient::new(rpc_url).await;

    match cli.command {
        Commands::CommitteeUpdate { slot, export } => {
            println!("SyncCommittee command received with slot: {}", slot);
            let proof = bankai.get_sync_committee_update(slot).await?;
            let json = serde_json::to_string_pretty(&proof)
                .map_err(|e| Error::DeserializeError(e.to_string()))?;

            if let Some(path) = export {
                match std::fs::write(path.clone(), json) {
                    Ok(_) => println!("Proof exported to {}", path),
                    Err(e) => return Err(Error::IoError(e)),
                }
            } else {
                println!("{}", json);
            }
        }
        Commands::EpochUpdate { slot, export } => {
            println!("Epoch command received with slot: {}", slot);
            let proof = bankai.get_epoch_proof(slot).await?;
            let json = serde_json::to_string_pretty(&proof)
                .map_err(|e| Error::DeserializeError(e.to_string()))?;

            if let Some(path) = export {
                match std::fs::write(path.clone(), json) {
                    Ok(_) => println!("Proof exported to {}", path),
                    Err(e) => return Err(Error::IoError(e)),
                }
            } else {
                println!("{}", json);
            }
        }
        Commands::ContractInit { slot, export } => {
            println!("ContractInit command received with slot: {}", slot);
            let contract_init = bankai
                .get_contract_initialization_data(slot, &bankai.config)
                .await?;
            let json = serde_json::to_string_pretty(&contract_init)
                .map_err(|e| Error::DeserializeError(e.to_string()))?;

            if let Some(path) = export {
                match std::fs::write(path.clone(), json) {
                    Ok(_) => println!("Contract initialization data exported to {}", path),
                    Err(e) => return Err(Error::IoError(e)),
                }
            } else {
                println!("{}", json);
            }
        }
        Commands::DeployContract { slot } => {
            let contract_init = bankai
                .get_contract_initialization_data(slot, &bankai.config)
                .await?;
            bankai
                .starknet_client
                .deploy_contract(contract_init, &bankai.config)
                .await?;
            // bankai.starknet_client.get_committee_hash(slot, &bankai.config).await?;
        }
        Commands::SubmitEpochProof { slot } => {
            let proof = bankai.get_epoch_proof(slot).await?;
            let header_root = bankai.client.get_block_root(slot).await?;
            bankai
                .starknet_client
                .submit_epoch_update(proof, header_root, &bankai.config)
                .await?;
            // bankai.starknet_client.get_latest_epoch(&bankai.config).await?;
        }
        Commands::SubmitNextCommittee => {
            let latest_committee_id = bankai
                .starknet_client
                .get_latest_committee_id(&bankai.config)
                .await?;
            let next_committee_slot = (latest_committee_id) * Felt::from(0x2000);
            // println!("Next committee slot: {}", next_committee_slot);
            // let latest_epoch = bankai.starknet_client.get_latest_epoch(&bankai.config).await?;
            // println!("Latest epoch: {}", latest_epoch);
            // assert!(latest_epoch >= next_committee_slot, "Newer Epoch required for next committee update");
            // let proof = bankai.get_sync_committee_update(latest_epoch.try_into().unwrap()).await?;
            // println!("Proof: {:?}", proof);
            // let mut bytes = [0u8; 48];
            // bytes.copy_from_slice(proof.next_aggregate_sync_committee.as_slice());
            // let committee_hash = utils::hashing::get_committee_hash(G1Affine::from_compressed(&bytes).unwrap());
            // println!("committee_hash: {:?}", committee_hash);
            // let header_root = bankai.client.get_block_root(next_committee_slot).await?;
            // bankai.starknet_client.submit_committee_update(proof, header_root, &bankai.config).await?;
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

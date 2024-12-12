mod contract_init;
mod epoch_update;
mod sync_committee;
mod traits;
mod utils;

use contract_init::ContractInitializationData;
use epoch_update::EpochUpdate;
use utils::cairo_runner::{CairoRunner};
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
    RequiresNewerEpoch(Felt),
    CairoRunError(String),
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
    epoch_circuit_path: String,
    committee_circuit_path: String,
}

impl Default for BankaiConfig {
    fn default() -> Self {
        Self {
            contract_class_hash: Felt::from_hex(
                "0x052c05f5027ad8f963168ebdf9d1518c938648681e43edd00807c28a71ea0b6a",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0x3c36fad01f7a9a8e893e7983a80bffb9ff81079b30f56703cff75a2347d619f",
            )
            .unwrap(),
            committee_update_program_hash: Felt::from_hex(
                "0x229e5ad2e3b8c6dd4d0319cdd957bbd7bdf2ea685e172b049c3e5f55b0352c1",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x61c9a8dc4629396452bffa605c59c947a4a344d85c6496f591787f2b6c422db",
            )
            .unwrap(),
            contract_path: "../contract/target/release/bankai_BankaiContract.contract_class.json"
                .to_string(),
            epoch_circuit_path: "../cairo/build/epoch_update.json"
                .to_string(),
            committee_circuit_path: "../cairo/build/committee_update.json"
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
            starknet_client: StarknetClient::new("https://free-rpc.nethermind.io/sepolia-juno/v0_7").await.unwrap(),
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
    SubmitEpoch {
        #[arg(long, short)]
        slot: u64,
    },
    SubmitNextEpoch,
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
        Commands::SubmitEpoch { slot } => {
            let proof = bankai.get_epoch_proof(slot).await?;
            bankai
                .starknet_client
                .submit_update(proof.expected_circuit_outputs, &bankai.config)
                .await?;
        }
        Commands::SubmitNextEpoch => {
            let latest_epoch = bankai.starknet_client.get_latest_epoch(&bankai.config).await?;
            println!("Curr epoch: {}", latest_epoch);
            // make sure next_epoch % 32 == 0
            let next_epoch = (u64::try_from(latest_epoch).unwrap() / 32) * 32 + 32;
            println!("Next epoch: {}", next_epoch);
            let proof = bankai.get_epoch_proof(next_epoch).await?;
            CairoRunner::generate_pie(proof, &bankai.config)?;
            // bankai.starknet_client.submit_update(proof.expected_circuit_outputs, &bankai.config).await?;
        }
        Commands::SubmitNextCommittee => {
            let latest_committee_id = bankai
                .starknet_client
                .get_latest_committee_id(&bankai.config)
                .await?;
            let lowest_committee_update_slot = (latest_committee_id) * Felt::from(0x2000);
            println!("Min Slot Required: {}", lowest_committee_update_slot);
            let latest_epoch = bankai.starknet_client.get_latest_epoch(&bankai.config).await?;
            println!("Latest epoch: {}", latest_epoch);
            if latest_epoch < lowest_committee_update_slot {
                return Err(Error::RequiresNewerEpoch(latest_epoch));
            }
            let update = bankai.get_sync_committee_update(latest_epoch.try_into().unwrap()).await?;
            CairoRunner::generate_pie(update, &bankai.config)?;
            // bankai.starknet_client.submit_update(update.expected_circuit_outputs, &bankai.config).await?;
        }
    }

    Ok(())
}

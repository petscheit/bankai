mod config;
mod contract_init;
mod epoch_update;
mod sync_committee;
mod traits;
mod utils;

use clap::{Parser, Subcommand};
use config::BankaiConfig;
use contract_init::ContractInitializationData;
use dotenv::from_filename;
use epoch_update::EpochUpdate;
use starknet::core::types::Felt;
use std::env;
use sync_committee::SyncCommitteeUpdate;
use traits::Provable;
use utils::{atlantic_client::AtlanticClient, cairo_runner::CairoRunner};
use utils::{
    rpc::BeaconRpcClient,
    starknet_client::{StarknetClient, StarknetError},
};

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
    AtlanticError(reqwest::Error),
    InvalidResponse(String),
}

impl From<StarknetError> for Error {
    fn from(e: StarknetError) -> Self {
        Error::StarknetError(e)
    }
}

struct BankaiClient {
    client: BeaconRpcClient,
    starknet_client: StarknetClient,
    config: BankaiConfig,
    atlantic_client: AtlanticClient,
}

impl BankaiClient {
    pub async fn new() -> Self {
        from_filename(".env.sepolia").ok();
        let config = BankaiConfig::default();
        Self {
            client: BeaconRpcClient::new(env::var("BEACON_RPC_URL").unwrap()),
            starknet_client: StarknetClient::new(
                env::var("STARKNET_RPC_URL").unwrap().as_str(),
                env::var("STARKNET_ADDRESS").unwrap().as_str(),
                env::var("STARKNET_PRIVATE_KEY").unwrap().as_str(),
            )
            .await
            .unwrap(),
            atlantic_client: AtlanticClient::new(
                config.atlantic_endpoint.clone(),
                env::var("ATLANTIC_API_KEY").unwrap(),
            ),
            config,
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
    ProveNextCommittee,
    ProveNextEpoch,
    CheckBatchStatus {
        #[arg(long, short)]
        batch_id: String,
    },
    SubmitWrappedProof {
        #[arg(long, short)]
        batch_id: String,
    },
    VerifyEpoch {
        #[arg(long, short)]
        batch_id: String,
        #[arg(long, short)]
        slot: u64,
    },
    VerifyCommittee {
        #[arg(long, short)]
        batch_id: String,
        #[arg(long, short)]
        slot: u64,
    },
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
    // Load .env.sepolia file
    from_filename(".env.sepolia").ok();

    let cli = Cli::parse();
    let bankai = BankaiClient::new().await;

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
        }
        Commands::CheckBatchStatus { batch_id } => {
            let status = bankai
                .atlantic_client
                .check_batch_status(batch_id.as_str())
                .await?;
            println!("Batch Status: {}", status);
        }
        Commands::ProveNextCommittee => {
            let latest_committee_id = bankai
                .starknet_client
                .get_latest_committee_id(&bankai.config)
                .await?;
            let lowest_committee_update_slot = (latest_committee_id) * Felt::from(0x2000);
            println!("Min Slot Required: {}", lowest_committee_update_slot);
            let latest_epoch = bankai
                .starknet_client
                .get_latest_epoch(&bankai.config)
                .await?;
            println!("Latest epoch: {}", latest_epoch);
            if latest_epoch < lowest_committee_update_slot {
                return Err(Error::RequiresNewerEpoch(latest_epoch));
            }
            let update = bankai
                .get_sync_committee_update(latest_epoch.try_into().unwrap())
                .await?;
            CairoRunner::generate_pie(&update, &bankai.config)?;
            let batch_id = bankai.atlantic_client.submit_batch(update).await?;
            println!("Batch Submitted: {}", batch_id);
        }
        Commands::ProveNextEpoch => {
            let latest_epoch = bankai
                .starknet_client
                .get_latest_epoch(&bankai.config)
                .await?;
            println!("Latest Epoch: {}", latest_epoch);
            // make sure next_epoch % 32 == 0
            let next_epoch = (u64::try_from(latest_epoch).unwrap() / 32) * 32 + 32;
            println!("Fetching Inputs for Epoch: {}", next_epoch);
            let proof = bankai.get_epoch_proof(next_epoch).await?;
            CairoRunner::generate_pie(&proof, &bankai.config)?;
            let batch_id = bankai.atlantic_client.submit_batch(proof).await?;
            println!("Batch Submitted: {}", batch_id);
        }
        Commands::VerifyEpoch { batch_id, slot } => {
            let status = bankai
                .atlantic_client
                .check_batch_status(batch_id.as_str())
                .await?;
            if status == "DONE" {
                let update = EpochUpdate::from_json::<EpochUpdate>(slot)?;
                bankai
                    .starknet_client
                    .submit_update(update.expected_circuit_outputs, &bankai.config)
                    .await?;
                println!("Successfully submitted epoch update");
            } else {
                println!("Batch not completed yet. Status: {}", status);
            }
        }
        Commands::VerifyCommittee { batch_id, slot } => {
            let status = bankai
                .atlantic_client
                .check_batch_status(batch_id.as_str())
                .await?;
            if status == "DONE" {
                let update = SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(slot)?;
                bankai
                    .starknet_client
                    .submit_update(update.expected_circuit_outputs, &bankai.config)
                    .await?;
                println!("Successfully submitted sync committee update");
            } else {
                println!("Batch not completed yet. Status: {}", status);
            }
        }
        Commands::SubmitWrappedProof { batch_id } => {
            let status = bankai
                .atlantic_client
                .check_batch_status(batch_id.as_str())
                .await?;
            if status == "DONE" {
                let proof = bankai
                    .atlantic_client
                    .fetch_proof(batch_id.as_str())
                    .await?;
                let batch_id = bankai.atlantic_client.submit_wrapped_proof(proof).await?;
                println!("Batch Submitted: {}", batch_id);
            } else {
                println!("Batch not completed yet. Status: {}", status);
            }
        }
    }

    Ok(())
}

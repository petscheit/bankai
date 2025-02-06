#![allow(dead_code)]
#![allow(unused_imports)]
mod bankai_client;
mod config;
mod constants;
mod contract_init;
pub mod epoch_batch;
mod epoch_update;
mod execution_header;
mod helpers;
mod state;
mod sync_committee;
mod traits;
mod utils;

use beacon_state_proof::error::Error as BeaconStateProofError;
use config::BankaiConfig;
use contract_init::ContractInitializationData;
use epoch_batch::EpochUpdateBatch;
use epoch_update::EpochUpdate;
use execution_header::ExecutionHeaderProof;
use starknet::core::types::Felt;
use sync_committee::SyncCommitteeUpdate;
use traits::Provable;
use utils::{atlantic_client::AtlanticClient, cairo_runner::CairoRunner};
use utils::{
    rpc::BeaconRpcClient,
    starknet_client::{StarknetClient, StarknetError},
};

use bankai_client::BankaiClient;
// use rand::Rng;
// use std::fs::File;
// use std::io::Write;
use clap::{Parser, Subcommand};
use dotenv::from_filename;
use state::Error;
use std::env;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

// impl From<StarknetError> for Error {
//     fn from(e: StarknetError) -> Self {
//         Error::StarknetError(e)
//     }
// }

// struct BankaiClient {
//     client: BeaconRpcClient,
//     starknet_client: StarknetClient,
//     config: BankaiConfig,
//     atlantic_client: AtlanticClient,
// }

// impl BankaiClient {
//     pub async fn new() -> Self {
//         from_filename(".env.sepolia").ok();
//         let config = BankaiConfig::default();
//         Self {
//             client: BeaconRpcClient::new(env::var("BEACON_RPC_URL").unwrap()),
//             starknet_client: StarknetClient::new(
//                 env::var("STARKNET_RPC_URL").unwrap().as_str(),
//                 env::var("STARKNET_ADDRESS").unwrap().as_str(),
//                 env::var("STARKNET_PRIVATE_KEY").unwrap().as_str(),
//             )
//             .await
//             .unwrap(),
//             atlantic_client: AtlanticClient::new(
//                 config.atlantic_endpoint.clone(),
//                 env::var("ATLANTIC_API_KEY").unwrap(),
//             ),
//             config,
//         }
//     }

//     pub async fn get_sync_committee_update(
//         &self,
//         mut slot: u64,
//     ) -> Result<SyncCommitteeUpdate, Error> {
//         let mut attempts = 0;
//         const MAX_ATTEMPTS: u8 = 3;

//         // Before we start generating the proof, we ensure the slot was not missed
//         let _header = loop {
//             match self.client.get_header(slot).await {
//                 Ok(header) => break header,
//                 Err(Error::EmptySlotDetected(_)) => {
//                     attempts += 1;
//                     if attempts >= MAX_ATTEMPTS {
//                         return Err(Error::EmptySlotDetected(slot));
//                     }
//                     slot += 1;
//                     println!(
//                         "Empty slot detected! Attempt {}/{}. Fetching slot: {}",
//                         attempts, MAX_ATTEMPTS, slot
//                     );
//                 }
//                 Err(e) => return Err(e), // Propagate other errors immediately
//             }
//         };

//         let proof: SyncCommitteeUpdate = SyncCommitteeUpdate::new(&self.client, slot).await?;

//         Ok(proof)
//     }

//     pub async fn get_epoch_proof(&self, slot: u64) -> Result<EpochUpdate, Error> {
//         let epoch_proof = EpochUpdate::new(&self.client, slot).await?;
//         Ok(epoch_proof)
//     }

//     pub async fn get_contract_initialization_data(
//         &self,
//         slot: u64,
//         config: &BankaiConfig,
//     ) -> Result<ContractInitializationData, Error> {
//         let contract_init = ContractInitializationData::new(&self.client, slot, config).await?;
//         Ok(contract_init)
//     }
// }

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
    ProveNextEpochBatch,
    ProveCommitteeAtSlot {
        #[arg(long, short)]
        slot: u64,
    },
    CheckBatchStatus {
        #[arg(long, short)]
        batch_id: String,
    },
    SubmitWrappedProof {
        #[arg(long, short)]
        batch_id: String,
    },
    GetEpochProof {
        #[arg(long, short)]
        epoch_id: u64,
    },
    VerifyEpoch {
        #[arg(long, short)]
        batch_id: String,
        #[arg(long, short)]
        slot: u64,
    },
    VerifyEpochBatch {
        #[arg(long, short)]
        batch_id: String,
        #[arg(long, short)]
        first_slot: u64,
        #[arg(long, short)]
        last_slot: u64,
    },
    VerifyCommittee {
        #[arg(long, short)]
        batch_id: String,
        #[arg(long, short)]
        slot: u64,
    },
    ExecutionHeader {
        #[arg(long, short)]
        block: u64,
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

    let subscriber = FmtSubscriber::builder()
        // .with_max_level(Level::TRACE)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();
    let bankai = BankaiClient::new().await;

    match cli.command {
        Commands::ExecutionHeader { block } => {
            let proof = ExecutionHeaderProof::fetch_proof(&bankai.client, block).await?;
            let json = serde_json::to_string_pretty(&proof)
                .map_err(|e| Error::DeserializeError(e.to_string()))?;
            println!("{}", json);
        }
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
            let latest_epoch_slot = bankai
                .starknet_client
                .get_latest_epoch_slot(&bankai.config)
                .await?;
            println!("Latest epoch slot: {}", latest_epoch_slot);
            if latest_epoch_slot < lowest_committee_update_slot {
                return Err(Error::RequiresNewerEpoch(latest_epoch_slot));
            }
            let update = bankai
                .get_sync_committee_update(latest_epoch_slot.try_into().unwrap())
                .await?;
            let _ = update.export()?;
            CairoRunner::generate_pie(&update, &bankai.config, None, None).await?;
            let batch_id = bankai.atlantic_client.submit_batch(update).await?;
            println!("Batch Submitted: {}", batch_id);
        }
        Commands::ProveNextEpoch => {
            let latest_epoch = bankai
                .starknet_client
                .get_latest_epoch_slot(&bankai.config)
                .await?;
            println!("Latest Epoch: {}", latest_epoch);
            // make sure next_epoch % 32 == 0
            let next_epoch = (u64::try_from(latest_epoch).unwrap() / 32) * 32 + 32;
            println!("Fetching Inputs for Epoch: {}", next_epoch);
            // let proof = bankai.get_epoch_proof(next_epoch).await?;
            let epoch_update = EpochUpdate::new(&bankai.client, next_epoch).await?;
            let _ = epoch_update.export()?;
            CairoRunner::generate_pie(&epoch_update, &bankai.config, None, None).await?;
            let batch_id = bankai.atlantic_client.submit_batch(epoch_update).await?;
            println!("Batch Submitted: {}", batch_id);
        }
        Commands::ProveNextEpochBatch => {
            let epoch_update = EpochUpdateBatch::new(&bankai).await?;
            println!("Update contents: {:?}", epoch_update);
            let _ = epoch_update.export()?;
            CairoRunner::generate_pie(&epoch_update, &bankai.config, None, None).await?;
            let batch_id = bankai.atlantic_client.submit_batch(epoch_update).await?;
            println!("Batch Submitted: {}", batch_id);
        }
        Commands::ProveCommitteeAtSlot { slot } => {
            let latest_committee_id = bankai
                .starknet_client
                .get_latest_committee_id(&bankai.config)
                .await?;
            let lowest_committee_update_slot = (latest_committee_id) * Felt::from(0x2000);
            println!("Min Slot Required: {}", lowest_committee_update_slot);
            // if slot < lowest_committee_update_slot {
            //     return Err(Error::RequiresNewerEpoch(slot));
            // }
            let update = bankai
                .get_sync_committee_update(slot.try_into().unwrap())
                .await?;
            let _ = update.export()?;
            CairoRunner::generate_pie(&update, &bankai.config, None, None).await?;
            let batch_id = bankai.atlantic_client.submit_batch(update).await?;
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
        Commands::VerifyEpochBatch {
            batch_id,
            first_slot,
            last_slot,
        } => {
            let status = bankai
                .atlantic_client
                .check_batch_status(batch_id.as_str())
                .await?;
            if status == "DONE" {
                let update =
                    EpochUpdateBatch::from_json::<EpochUpdateBatch>(first_slot, last_slot)?;
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
        Commands::GetEpochProof { epoch_id } => {
            let epoch_proof = bankai
                .starknet_client
                .get_epoch_proof(epoch_id, &bankai.config)
                .await?;

            println!("Retrieved epoch proof from contract: {:?}", epoch_proof);
        }
    }

    Ok(())
}

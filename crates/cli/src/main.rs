use bankai_core::cairo_runner::CairoError;
use bankai_core::cairo_runner::{
    generate_committee_update_pie, generate_epoch_batch_pie, generate_epoch_update_pie,
};
use bankai_core::clients::atlantic::AtlanticError;
use bankai_core::clients::starknet::StarknetError;
use bankai_core::types::error::BankaiCoreError;
use bankai_core::types::proofs::epoch_batch::EpochUpdateBatch;
use bankai_core::types::proofs::epoch_update::EpochUpdate;
use bankai_core::types::proofs::execution_header::ExecutionHeaderProof;
use bankai_core::types::proofs::sync_committee::SyncCommitteeUpdate;
use bankai_core::types::proofs::ProofError;
use bankai_core::types::traits::Exportable;
use bankai_core::types::traits::ProofType;
use bankai_core::BankaiClient;
use clap::{Parser, Subcommand};
use dotenv::from_filename;
use starknet::core::types::Felt;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Subcommand)]
enum Commands {
    /// Generate and manage proofs for the light client state
    #[command(subcommand)]
    Prove(ProveCommands),

    /// Fetch proof data from the network
    #[command(subcommand)]
    Fetch(FetchCommands),

    /// Generate and manage contract data
    #[command(subcommand)]
    Contract(ContractCommands),

    /// Verify and submit proofs to the network
    #[command(subcommand)]
    Verify(VerifyCommands),

    /// Query and check proof status
    #[command(subcommand)]
    Status(StatusCommands),
}

#[derive(Subcommand)]
enum FetchCommands {
    /// Fetch a sync committee update proof for a given slot
    CommitteeUpdate {
        /// The slot number to generate the proof for
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    /// Fetch an epoch update proof for a given slot
    EpochUpdate {
        /// The slot number to generate the proof for
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    /// Fetch an execution header proof for a given block
    ExecutionHeader {
        /// The block number to generate the proof for
        #[arg(long, short)]
        block: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProveCommands {
    /// Generate proof for the next committee update
    NextCommittee,
    /// Generate proof for the next epoch update
    NextEpoch,
    /// Generate proof for the next epoch batch
    NextEpochBatch,
    /// Generate proof for a committee at a specific slot
    CommitteeAtSlot {
        /// The slot number to generate the proof for
        #[arg(long, short)]
        slot: u64,
    },
    /// Submit a wrapped proof for verification
    SubmitWrapped {
        /// The batch ID of the proof to wrap and submit
        #[arg(long, short)]
        batch_id: String,
    },
}

#[derive(Subcommand)]
enum ContractCommands {
    /// Generate contract initialization data for a given slot
    Init {
        /// The slot number to generate initialization data for
        #[arg(long, short)]
        slot: u64,
        /// Export output to a JSON file
        #[arg(long, short)]
        export: Option<String>,
    },
    /// Deploy the contract with initialization data for a given slot
    Deploy {
        /// The slot number to deploy the contract for
        #[arg(long, short)]
        slot: u64,
    },
}

#[derive(Subcommand)]
enum VerifyCommands {
    /// Verify and submit an epoch update proof
    Epoch {
        /// The batch ID of the proof to verify
        #[arg(long, short)]
        batch_id: String,
        /// The slot number of the epoch update
        #[arg(long, short)]
        slot: u64,
    },
    /// Verify and submit an epoch batch proof
    EpochBatch {
        /// The batch ID of the proof to verify
        #[arg(long, short)]
        batch_id: String,
        /// The first epoch in the batch
        #[arg(long, short)]
        start_epoch: u64,
        /// The last epoch in the batch
        #[arg(long, short)]
        end_epoch: u64,
    },
    /// Verify and submit a committee update proof
    Committee {
        /// The batch ID of the proof to verify
        #[arg(long, short)]
        batch_id: String,
        /// The slot number of the committee update
        #[arg(long, short)]
        slot: u64,
    },
}

#[derive(Subcommand)]
enum StatusCommands {
    /// Check the status of a proof batch
    CheckBatch {
        /// The batch ID to check status for
        #[arg(long, short)]
        batch_id: String,
    },
    /// Get the proof for a specific epoch
    GetEpoch {
        /// The epoch ID to get the proof for
        #[arg(long, short)]
        epoch_id: u64,
    },
}

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Bankai CLI - Ethereum Light Client for Starknet",
    long_about = "A command-line interface for managing the Bankai Ethereum Light Client on Starknet. \
                  This tool helps generate, verify, and manage proofs for epoch and sync committee updates."
)]
struct Cli {
    /// Optional RPC URL (defaults to RPC_URL_BEACON environment variable)
    #[arg(long, short)]
    rpc_url: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<(), BankaiCliError> {
    // Load .env.sepolia file
    from_filename(".env.sepolia").ok();

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let cli = Cli::parse();
    let bankai = BankaiClient::new().await;

    match cli.command {
        Commands::Prove(cmd) => match cmd {
            ProveCommands::NextCommittee => {
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
                    return Err(BankaiCliError::RequiresNewerEpoch(latest_epoch_slot));
                }
                let update: SyncCommitteeUpdate = bankai
                    .get_sync_committee_update(latest_epoch_slot.try_into().unwrap())
                    .await?;
                let name = update.name();
                let pie = generate_committee_update_pie(update, &bankai.config, None, None).await?;
                let batch_id = bankai
                    .atlantic_client
                    .submit_batch(pie, ProofType::SyncCommittee, name)
                    .await?;
                println!("Batch Submitted: {}", batch_id);
            }
            ProveCommands::NextEpoch => {
                let latest_epoch = bankai
                    .starknet_client
                    .get_latest_epoch_slot(&bankai.config)
                    .await?;
                println!("Latest Epoch: {}", latest_epoch);
                // make sure next_epoch % 32 == 0
                let next_epoch = (u64::try_from(latest_epoch).unwrap() / 32) * 32 + 32;
                println!("Fetching Inputs for Epoch: {}", next_epoch);
                let epoch_update = EpochUpdate::new(&bankai.client, next_epoch)
                    .await
                    .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;
                
                let export_path = epoch_update.export()?;
                println!("Update exported to {}", export_path);
                let pie =
                    generate_epoch_update_pie(epoch_update, &bankai.config, None, None).await?;
                let batch_id = bankai
                    .atlantic_client
                    .submit_batch(pie, ProofType::Epoch, format!("epoch_{}", next_epoch))
                    .await?;
                println!("Batch Submitted: {}", batch_id);
            }
            ProveCommands::NextEpochBatch => {
                let epoch_update = EpochUpdateBatch::new(&bankai)
                    .await
                    .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;
                let name = epoch_update.name();
                let export_path = epoch_update.export()?;
                println!("Update exported to {}", export_path);
                let pie =
                    generate_epoch_batch_pie(epoch_update, &bankai.config, None, None).await?;
                let batch_id = bankai
                    .atlantic_client
                    .submit_batch(pie, ProofType::EpochBatch, name)
                    .await?;
                println!("Batch Submitted: {}", batch_id);
            }
            ProveCommands::CommitteeAtSlot { slot } => {
                let latest_committee_id = bankai
                    .starknet_client
                    .get_latest_committee_id(&bankai.config)
                    .await?;
                let lowest_committee_update_slot = (latest_committee_id) * Felt::from(0x2000);
                println!("Min Slot Required: {}", lowest_committee_update_slot);
                let update = bankai
                    .get_sync_committee_update(slot.try_into().unwrap())
                    .await?;
                let name = update.name();
                let export_path = update.export()?;
                println!("Update exported to {}", export_path);
                let pie = generate_committee_update_pie(update, &bankai.config, None, None).await?;
                let batch_id = bankai
                    .atlantic_client
                    .submit_batch(pie, ProofType::SyncCommittee, name)
                    .await?;
                println!("Batch Submitted: {}", batch_id);
            }
            ProveCommands::SubmitWrapped { batch_id } => {
                let status = bankai
                    .atlantic_client
                    .check_batch_status(batch_id.as_str())
                    .await?;
                if status == "DONE" {
                    let proof = bankai
                        .atlantic_client
                        .fetch_proof(batch_id.as_str())
                        .await?;
                    let batch_id = bankai
                        .atlantic_client
                        .submit_wrapped_proof(proof, bankai.config.cairo_verifier_path, format!("wrap_{}", batch_id))
                        .await?;
                    println!("Batch Submitted: {}", batch_id);
                } else {
                    println!("Batch not completed yet. Status: {}", status);
                }
            }
        },
        Commands::Fetch(cmd) => match cmd {
            FetchCommands::ExecutionHeader { block, export } => {
                let proof = ExecutionHeaderProof::fetch_proof(&bankai.client, block)
                    .await
                    .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;
                let json = serde_json::to_string_pretty(&proof)?;

                if let Some(path) = export {
                    match std::fs::write(path.clone(), json) {
                        Ok(_) => println!("Proof exported to {}", path),
                        Err(e) => return Err(BankaiCliError::IoError(e)),
                    }
                } else {
                    println!("{}", json);
                }
            }
            FetchCommands::CommitteeUpdate { slot, export } => {
                println!("SyncCommittee command received with slot: {}", slot);
                let proof = bankai.get_sync_committee_update(slot).await?;
                let json = serde_json::to_string_pretty(&proof)?;

                if let Some(path) = export {
                    match std::fs::write(path.clone(), json) {
                        Ok(_) => println!("Proof exported to {}", path),
                        Err(e) => return Err(BankaiCliError::IoError(e)),
                    }
                } else {
                    println!("{}", json);
                }
            }
            FetchCommands::EpochUpdate { slot, export } => {
                println!("Epoch command received with slot: {}", slot);
                let proof = bankai.get_epoch_proof(slot).await?;
                let json = serde_json::to_string_pretty(&proof)?;

                if let Some(path) = export {
                    match std::fs::write(path.clone(), json) {
                        Ok(_) => println!("Proof exported to {}", path),
                        Err(e) => return Err(BankaiCliError::IoError(e)),
                    }
                } else {
                    println!("{}", json);
                }
            }
        },
        Commands::Contract(cmd) => match cmd {
            ContractCommands::Init { slot, export } => {
                println!("ContractInit command received with slot: {}", slot);
                let contract_init = bankai
                    .get_contract_initialization_data(slot, &bankai.config)
                    .await?;
                let json = serde_json::to_string_pretty(&contract_init)?;

                if let Some(path) = export {
                    match std::fs::write(path.clone(), json) {
                        Ok(_) => println!("Contract initialization data exported to {}", path),
                        Err(e) => return Err(BankaiCliError::IoError(e)),
                    }
                } else {
                    println!("{}", json);
                }
            }
            ContractCommands::Deploy { slot } => {
                let contract_init = bankai
                    .get_contract_initialization_data(slot, &bankai.config)
                    .await?;
                bankai
                    .starknet_client
                    .deploy_contract(contract_init, &bankai.config)
                    .await?;
            }
        },
        Commands::Verify(cmd) => match cmd {
            VerifyCommands::Epoch { batch_id, slot } => {
                let status = bankai
                    .atlantic_client
                    .check_batch_status(batch_id.as_str())
                    .await?;
                if status == "DONE" {
                    let update = EpochUpdate::from_json::<EpochUpdate>(slot)
                        .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;
                    bankai
                        .starknet_client
                        .submit_update(update.expected_circuit_outputs, &bankai.config)
                        .await?;
                    println!("Successfully submitted epoch update");
                } else {
                    println!("Batch not completed yet. Status: {}", status);
                }
            }
            VerifyCommands::EpochBatch {
                batch_id,
                start_epoch,
                end_epoch,
            } => {
                let status = bankai
                    .atlantic_client
                    .check_batch_status(batch_id.as_str())
                    .await?;
                if status == "DONE" {
                    let update =
                        EpochUpdateBatch::from_json::<EpochUpdateBatch>(start_epoch, end_epoch)
                            .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;

                    bankai
                        .starknet_client
                        .submit_update(update.expected_circuit_outputs, &bankai.config)
                        .await?;
                    println!("Successfully submitted epoch update");
                } else {
                    println!("Batch not completed yet. Status: {}", status);
                }
            }
            VerifyCommands::Committee { batch_id, slot } => {
                let status = bankai
                    .atlantic_client
                    .check_batch_status(batch_id.as_str())
                    .await?;
                if status == "DONE" {
                    let update = SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(slot)
                        .map_err(|e| BankaiCliError::ProofFetch(e.into()))?;
                    bankai
                        .starknet_client
                        .submit_update(update.expected_circuit_outputs, &bankai.config)
                        .await?;
                    println!("Successfully submitted sync committee update");
                } else {
                    println!("Batch not completed yet. Status: {}", status);
                }
            }
        },
        Commands::Status(cmd) => match cmd {
            StatusCommands::CheckBatch { batch_id } => {
                let status = bankai
                    .atlantic_client
                    .check_batch_status(batch_id.as_str())
                    .await?;
                println!("Batch Status: {}", status);
            }
            StatusCommands::GetEpoch { epoch_id } => {
                let epoch_proof = bankai
                    .starknet_client
                    .get_epoch_proof(epoch_id, &bankai.config)
                    .await?;

                println!("Retrieved epoch proof from contract: {:?}", epoch_proof);
            }
        },
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum BankaiCliError {
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Bankai atlantic error: {0}")]
    Atlantic(#[from] AtlanticError),
    #[error("Bankai Starknet error: {0}")]
    Starknet(#[from] StarknetError),
    #[error("Bankai core error: {0}")]
    Core(#[from] BankaiCoreError),
    #[error("Requires newer epoch: {0}")]
    RequiresNewerEpoch(Felt),
    #[error("Proof fetch error: {0}")]
    ProofFetch(#[from] ProofError),
    #[error("Cairo runner error: {0}")]
    CairoRunner(#[from] CairoError),
}

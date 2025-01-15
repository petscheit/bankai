use crate::bankai_client::BankaiClient;
use crate::utils::starknet_client::StarknetError;
use crate::utils::{
    database_manager::DatabaseManager,
};
use postgres_types::{FromSql, ToSql};
use starknet::core::types::Felt;
use std::env;
use std::fmt;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_postgres::Client;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Job {
    pub job_id: Uuid,
    pub job_type: JobType,
    pub job_status: JobStatus,
    pub slot: u64,
}

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub tx: mpsc::Sender<Job>,
    pub bankai: Arc<BankaiClient>,
}

#[derive(Debug, FromSql, ToSql, Clone)]
#[postgres(name = "job_status")]
pub enum JobStatus {
    #[postgres(name = "CREATED")]
    Created,
    #[postgres(name = "FETCHED_PROOF")]
    FetchedProof,
    #[postgres(name = "PIE_GENERATED")]
    PieGenerated,
    #[postgres(name = "OFFCHAIN_PROOF_REQUESTED")]
    OffchainProofRequested,
    #[postgres(name = "OFFCHAIN_PROOF_RETRIEVED")]
    OffchainProofRetrieved,
    #[postgres(name = "WRAP_PROOF_REQUESTED")]
    WrapProofRequested,
    #[postgres(name = "WRAPPED_PROOF_DONE")]
    WrappedProofDone,
    #[postgres(name = "READY_TO_BROADCAST")]
    ReadyToBroadcast,
    #[postgres(name = "PROOF_VERIFY_CALLED_ONCHAIN")]
    ProofVerifyCalledOnchain,
    #[postgres(name = "VERIFIED_FACT_REGISTERED")]
    VerifiedFactRegistered,
    #[postgres(name = "ERROR")]
    Error,
    #[postgres(name = "CANCELLED")]
    Cancelled,
}

impl ToString for JobStatus {
    fn to_string(&self) -> String {
        match self {
            JobStatus::Created => "CREATED".to_string(),
            JobStatus::FetchedProof => "FETCHED_PROOF".to_string(),
            JobStatus::PieGenerated => "PIE_GENERATED".to_string(),
            JobStatus::OffchainProofRequested => "OFFCHAIN_PROOF_REQUESTED".to_string(),
            JobStatus::OffchainProofRetrieved => "OFFCHAIN_PROOF_RETRIEVED".to_string(),
            JobStatus::WrapProofRequested => "WRAP_PROOF_REQUESTED".to_string(),
            JobStatus::WrappedProofDone => "WRAPPED_PROOF_DONE".to_string(),
            JobStatus::ReadyToBroadcast => "READY_TO_BROADCAST".to_string(),
            JobStatus::ProofVerifyCalledOnchain => "PROOF_VERIFY_CALLED_ONCHAIN".to_string(),
            JobStatus::VerifiedFactRegistered => "VERIFIED_FACT_REGISTERED".to_string(),
            JobStatus::Cancelled => "CANCELLED".to_string(),
            JobStatus::Error => "ERROR".to_string(),
        }
    }
}

#[derive(Debug, FromSql, ToSql, Clone)]
pub enum JobType {
    EpochUpdate,
    EpochBatchUpdate,
    SyncComiteeUpdate,
}

#[derive(Debug, FromSql, ToSql)]
pub enum AtlanticJobType {
    ProofGeneration,
    ProofWrapping,
}

// Checking status of env vars
pub fn check_env_vars() -> Result<(), String> {
    let required_vars = [
        "BEACON_RPC_URL",
        "STARKNET_RPC_URL",
        "STARKNET_ADDRESS",
        "STARKNET_PRIVATE_KEY",
        "ATLANTIC_API_KEY",
        "PROOF_REGISTRY",
        "POSTGRESQL_HOST",
        "POSTGRESQL_USER",
        "POSTGRESQL_PASSWORD",
        "POSTGRESQL_DB_NAME",
        "RPC_LISTEN_HOST",
        "RPC_LISTEN_PORT",
    ];

    for &var in &required_vars {
        if env::var(var).is_err() {
            return Err(format!("Environment variable `{}` is not set", var));
        }
    }

    Ok(())
}

/// Errors types

impl std::fmt::Display for StarknetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StarknetError::ProviderError(err) => write!(f, "Provider error: {}", err),
            StarknetError::AccountError(msg) => write!(f, "Account error: {}", msg),
        }
    }
}

impl std::error::Error for StarknetError {}

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
    PoolingTimeout(String),
    InvalidMerkleTree,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidProof => write!(f, "Invalid proof provided"),
            Error::RpcError(err) => write!(f, "RPC error: {}", err),
            Error::DeserializeError(msg) => write!(f, "Deserialization error: {}", msg),
            Error::IoError(err) => write!(f, "I/O error: {}", err),
            Error::StarknetError(err) => write!(f, "Starknet error: {}", err),
            Error::BlockNotFound => write!(f, "Block not found"),
            Error::FetchSyncCommitteeError => write!(f, "Failed to fetch sync committee"),
            Error::FailedFetchingBeaconState => write!(f, "Failed to fetch beacon state"),
            Error::InvalidBLSPoint => write!(f, "Invalid BLS point"),
            Error::MissingRpcUrl => write!(f, "Missing RPC URL"),
            Error::EmptySlotDetected(slot) => write!(f, "Empty slot detected: {}", slot),
            Error::RequiresNewerEpoch(felt) => write!(f, "Requires newer epoch: {}", felt),
            Error::CairoRunError(msg) => write!(f, "Cairo run error: {}", msg),
            Error::AtlanticError(err) => write!(f, "Atlantic RPC error: {}", err),
            Error::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Error::PoolingTimeout(msg) => write!(f, "Pooling timeout: {}", msg),
            Error::InvalidMerkleTree => write!(f, "Invalid Merkle Tree"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::RpcError(err) => Some(err),
            Error::IoError(err) => Some(err),
            Error::StarknetError(err) => Some(err),
            Error::AtlanticError(err) => Some(err),
            _ => None, // No underlying source for other variants
        }
    }
}

impl From<StarknetError> for Error {
    fn from(e: StarknetError) -> Self {
        Error::StarknetError(e)
    }
}

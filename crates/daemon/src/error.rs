use bankai_core::types::job::Job;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Failed to create job: {0}")]
    FailedToCreateJob(String),
    #[error("Database error: {0}")]
    Database(#[from] bankai_core::db::manager::DatabaseError),
    #[error("Atlantic error: {0}")]
    Atlantic(#[from] bankai_core::clients::atlantic::AtlanticError),
    #[error("Starknet error: {0}")]
    Starknet(#[from] bankai_core::clients::starknet::StarknetError),
    #[error("Proof type error: {0}")]
    ProofType(#[from] bankai_core::types::proofs::ProofError),
    #[error("Job error: {0}")]
    JobError(String),
    #[error("Bankai core error: {0}")]
    BankaiCore(#[from] bankai_core::types::error::BankaiCoreError),
    #[error("Cairo runner error: {0}")]
    CairoRunner(#[from] bankai_core::cairo_runner::CairoError),
    #[error("Send error: {0}")]
    Send(#[from] tokio::sync::mpsc::error::SendError<Job>),
    #[error("Transactor error: {0}")]
    Transactor(#[from] bankai_core::clients::transactor::TransactorError),
    #[error("Proof wrapping failed: {0}")]
    ProofWrappingFailed(String),
    #[error("Offchain proof job failed: {0}")]
    OffchainProofFailed(String),
}

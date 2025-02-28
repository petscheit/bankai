use thiserror::Error;

pub mod config;
pub mod constants;
pub mod hashing;
pub mod helpers;
pub mod merkle;

#[derive(Debug, Error)]
pub enum UtilsError {
    #[error("Merkle error: {0}")]
    Merkle(#[from] merkle::MerkleError),
}

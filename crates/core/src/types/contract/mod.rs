pub mod contract_init;

use contract_init::ContractInitializationError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("Contract initialization error: {0}")]
    Initialization(#[from] ContractInitializationError),
}

use thiserror::Error;

pub mod atlantic;
pub mod beacon_chain;
pub mod starknet;
pub mod transactor;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Beacon chain error: {0}")]
    Beacon(#[from] beacon_chain::BeaconError),
    #[error("Atlantic error: {0}")]
    Atlantic(#[from] atlantic::AtlanticError),
    #[error("Starknet error: {0}")]
    Starknet(#[from] starknet::StarknetError),
    #[error("Transactor error: {0}")]
    Transactor(#[from] transactor::TransactorError),
}

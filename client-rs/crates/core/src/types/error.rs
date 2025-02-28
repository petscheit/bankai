
// impl std::fmt::Display for StarknetError {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             StarknetError::ProviderError(err) => write!(f, "Provider error: {}", err),
//             StarknetError::AccountError(msg) => write!(f, "Account error: {}", msg),
//             StarknetError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
//             StarknetError::TimeoutError => {
//                 write!(f, "Waiting for transaction timeout error")
//             }
//         }
//     }
// }

// impl std::error::Error for StarknetError {}
use starknet::core::types::Felt;

#[allow(unused)]
#[derive(Debug)]
pub enum Error {
    InvalidProof,
    Other(String),
    RpcError(reqwest::Error),
    DeserializeError(String),
    IoError(std::io::Error),
    // StarknetError(StarknetError),
    BlockNotFound,
    FetchSyncCommitteeError,
    FailedFetchingBeaconState,
    InvalidBLSPoint,
    MissingRpcUrl,
    EmptySlotDetected(u64),
    RequiresNewerEpoch(Felt),
    CairoRunError(String),
    AtlanticError(reqwest::Error),
    AtlanticProcessingError(String),
    AtlanticPoolingTimeout(String),
    BankaiRPCClientError(reqwest::Error),
    InvalidResponse(String),
    PoolingTimeout(String),
    InvalidMerkleTree,
    DatabaseError(String),
    TransactorError(reqwest::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidProof => write!(f, "Invalid proof provided"),
            Error::RpcError(err) => write!(f, "RPC error: {}", err),
            Error::DeserializeError(msg) => write!(f, "Deserialization error: {}", msg),
            Error::IoError(err) => write!(f, "I/O error: {}", err),
            // Error::StarknetError(err) => write!(f, "Starknet error: {}", err),
            Error::BlockNotFound => write!(f, "Block not found"),
            Error::FetchSyncCommitteeError => write!(f, "Failed to fetch sync committee"),
            Error::FailedFetchingBeaconState => write!(f, "Failed to fetch beacon state"),
            Error::InvalidBLSPoint => write!(f, "Invalid BLS point"),
            Error::MissingRpcUrl => write!(f, "Missing RPC URL"),
            Error::EmptySlotDetected(slot) => write!(f, "Empty slot detected: {}", slot),
            Error::RequiresNewerEpoch(felt) => write!(f, "Requires newer epoch: {}", felt),
            Error::CairoRunError(msg) => write!(f, "Cairo run error: {}", msg),
            Error::AtlanticError(err) => write!(f, "Atlantic RPC error: {}", err),
            Error::AtlanticProcessingError(err) => {
                write!(f, "Atlantic query processing error: {}", err)
            }
            Error::AtlanticPoolingTimeout(err) => {
                write!(f, "Atlantic query processing error: {}", err)
            }
            Error::BankaiRPCClientError(err) => write!(f, "Bankai RPC client error: {}", err),
            Error::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            Error::PoolingTimeout(msg) => write!(f, "Pooling timeout: {}", msg),
            Error::InvalidMerkleTree => write!(f, "Invalid Merkle Tree"),
            Error::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Error::TransactorError(msg) => write!(f, "Transactor error: {}", msg),
            Error::Other(msg) => write!(f, "Other Error; {:?}", msg)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::RpcError(err) => Some(err),
            Error::IoError(err) => Some(err),
            // Error::StarknetError(err) => Some(err),
            Error::AtlanticError(err) => Some(err),
            Error::BankaiRPCClientError(err) => Some(err),

            _ => None, // No underlying source for other variants
        }
    }
}

// impl From<StarknetError> for Error {
//     fn from(e: StarknetError) -> Self {
//         Error::StarknetError(e)
//     }
// }

pub mod epoch_batch;
pub mod epoch_update;
pub mod execution_header;
pub mod sync_committee;

use epoch_batch::EpochBatchError;
use epoch_update::EpochUpdateError;
use execution_header::ExecutionHeaderError;
use sync_committee::SyncCommitteeError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProofError {
    #[error("Epoch update error: {0}")]
    EpochUpdate(#[from] EpochUpdateError),
    #[error("Epoch batch error: {0}")]
    EpochBatch(#[from] EpochBatchError),
    #[error("Sync committee error: {0}")]
    SyncCommittee(#[from] SyncCommitteeError),
    #[error("Execution header error: {0}")]
    ExecutionHeader(#[from] ExecutionHeaderError),
}

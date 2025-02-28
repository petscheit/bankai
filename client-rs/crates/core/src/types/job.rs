use postgres_types::{FromSql, ToSql};
use serde::Serialize;
use strum::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Job {
    pub job_id: Uuid,
    pub job_type: JobType,
    pub job_status: JobStatus,
    pub slot: Option<u64>,
    pub batch_range_begin_epoch: Option<u64>,
    pub batch_range_end_epoch: Option<u64>,
}

#[derive(Debug, FromSql, ToSql, Clone, Eq, Hash, PartialEq, Serialize, Display, EnumString)]
#[postgres(name = "job_status")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum JobStatus {
    #[postgres(name = "CREATED")]
    Created, // Can act as queued and be picked up by worker to proccess
    #[postgres(name = "PROGRAM_INPUTS_PREPARED")]
    ProgramInputsPrepared,
    #[postgres(name = "STARTED_FETCHING_INPUTS")]
    StartedFetchingInputs,
    #[postgres(name = "STARTED_TRACE_GENERATION")]
    StartedTraceGeneration,
    #[postgres(name = "PIE_GENERATED")]
    PieGenerated,
    #[postgres(name = "OFFCHAIN_PROOF_REQUESTED")]
    AtlanticProofRequested,
    #[postgres(name = "OFFCHAIN_PROOF_RETRIEVED")]
    AtlanticProofRetrieved,
    #[postgres(name = "WRAP_PROOF_REQUESTED")]
    WrapProofRequested,
    #[postgres(name = "WRAPPED_PROOF_DONE")]
    WrappedProofDone,
    #[postgres(name = "OFFCHAIN_COMPUTATION_FINISHED")]
    OffchainComputationFinished,
    #[postgres(name = "READY_TO_BROADCAST_ONCHAIN")]
    ReadyToBroadcastOnchain,
    #[postgres(name = "PROOF_VERIFY_CALLED_ONCHAIN")]
    ProofVerifyCalledOnchain,
    #[postgres(name = "DONE")]
    Done,
    #[postgres(name = "ERROR")]
    Error,
    #[postgres(name = "CANCELLED")]
    Cancelled,
}

#[derive(Debug, FromSql, ToSql, Clone, Serialize, Display, EnumString)]
#[postgres(name = "job_type")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum JobType {
    //EpochUpdate,
    #[postgres(name = "EPOCH_BATCH_UPDATE")]
    EpochBatchUpdate,
    #[postgres(name = "SYNC_COMMITTEE_UPDATE")]
    SyncCommitteeUpdate,
}

#[derive(Debug, FromSql, ToSql)]
pub enum AtlanticJobType {
    ProofGeneration,
    ProofWrapping,
}

use std::sync::Arc;

use crate::error::DaemonError;
use bankai_core::db::manager::DatabaseManager;
use bankai_core::types::job::{Job, JobStatus};
use tokio::sync::mpsc;
use tracing::info;

pub async fn update_job_status_for_resume(
    tx: mpsc::Sender<Job>,
    db_manager: Arc<DatabaseManager>,
    job: Job,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        info!(
            "[RETRY][{}] Current status: {}",
            job.job_id, job_data.job_status
        );

        let new_status = match job_data.job_status {
            JobStatus::Created
            | JobStatus::ProgramInputsPrepared
            | JobStatus::StartedFetchingInputs
            | JobStatus::StartedTraceGeneration
            | JobStatus::PieGenerated => JobStatus::Created,
            JobStatus::OffchainProofRetrieved => JobStatus::WrapProofRequested,
            JobStatus::WrappedProofDone => JobStatus::ReadyToBroadcastOnchain,
            JobStatus::ProofVerifyCalledOnchain => JobStatus::Done,
            _ => return Ok(()),
        };

        info!("[RETRY][{}] New status: {}", job.job_id, new_status);
        db_manager.update_job_status(job.job_id, new_status).await?;

        let job = db_manager
            .get_job_by_id(job.job_id)
            .await?
            .unwrap()
            .try_into()
            .unwrap();

        tx.send(job).await?;
    }

    Ok(())
}

use std::sync::Arc;

use bankai_core::db::manager::DatabaseManager;
use bankai_core::types::job::{Job, JobStatus, JobType};
use bankai_core::BankaiClient;
use tokio::sync::mpsc;
use tracing::info;
use crate::error::DaemonError;

pub async fn update_job_status_for_retry(tx: mpsc::Sender<Job>, db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>, job: Job) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        info!("[RETRY][{}] Current status: {}", job.job_id, job_data.job_status);
        let new_status =  if let Some(wrapper_id) = job_data.atlantic_proof_wrapper_batch_id {
            let atlantic_status = bankai.atlantic_client.check_batch_status(&wrapper_id).await?;
            if atlantic_status == "DONE" {
                JobStatus::OffchainComputationFinished
            } else {
                JobStatus::OffchainProofRequested
            }
        } else if let Some(batch_id) = job_data.atlantic_proof_generate_batch_id {
            let atlantic_status = bankai.atlantic_client.check_batch_status(&batch_id).await?;
            if atlantic_status == "DONE" {
                JobStatus::OffchainProofRequested
            } else {
                JobStatus::Created
            }
        } else {
            JobStatus::Created
        };

        info!("[RETRY][{}] New status: {}", job.job_id, new_status);
        let weight = match job_data.job_type {
            JobType::SyncCommitteeUpdate => 1,
            JobType::EpochBatchUpdate => match new_status {
                JobStatus::Created => 2,
                JobStatus::OffchainProofRequested => 4,
                JobStatus::OffchainComputationFinished => 1,
                _ => 1,
            },
            _ => 1,
        };
        db_manager.increase_job_retry_counter(job.job_id, weight).await?;
        db_manager.update_job_status(job.job_id, new_status).await?;
        let job = db_manager.get_job_by_id(job.job_id).await?.unwrap().try_into().unwrap();
        
        tx.send(job).await?;
    }
    Ok(())
}
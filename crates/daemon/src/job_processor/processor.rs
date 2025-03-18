use bankai_core::db::manager::DatabaseManager;
use bankai_core::types::job::{Job, JobStatus, JobType};
use bankai_core::BankaiClient;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;
// use crate::job_processor::epoch_batch::process_epoch_batch_job;

use super::broadcast::{broadcast_epoch_batch, broadcast_sync_committee};
use super::epoch_batch::EpochBatchJobProcessor;
use super::proof::{
    process_committee_wrapping_stage, process_epoch_batch_wrapping_stage,
    process_offchain_proof_stage,
};
use super::sync_committee::SyncCommitteeJobProcessor;
use crate::error::DaemonError;

#[derive(Clone)]
pub struct JobProcessor {
    pub db_manager: Arc<DatabaseManager>,
    pub bankai: Arc<BankaiClient>,
    pub sync_committee_job_processor: Arc<SyncCommitteeJobProcessor>,
    pub epoch_batch_job_processor: Arc<EpochBatchJobProcessor>,
}

impl JobProcessor {
    pub fn new(db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Self {
        Self {
            sync_committee_job_processor: Arc::new(SyncCommitteeJobProcessor::new(
                db_manager.clone(),
                bankai.clone(),
            )),
            epoch_batch_job_processor: Arc::new(EpochBatchJobProcessor::new(
                db_manager.clone(),
                bankai.clone(),
            )),
            db_manager,
            bankai,
        }
    }

    pub async fn process_trace_gen_job(&self, job: Job) -> Result<(), DaemonError> {
        match job.job_type {
            JobType::SyncCommitteeUpdate => {
                if let Err(e) = self
                    .sync_committee_job_processor
                    .process_job(job.clone())
                    .await
                {
                    self.handle_job_error(job.job_id).await?;
                    error!("Error processing sync committee update job: {:?}", e);
                    return Err(e);
                }
            }
            JobType::EpochBatchUpdate => {
                if let Err(e) = self
                    .epoch_batch_job_processor
                    .process_job(job.clone())
                    .await
                {
                    self.handle_job_error(job.job_id).await?;
                    error!("Error processing epoch batch update job: {:?}", e);
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    pub async fn process_proof_job(&self, job: Job) -> Result<(), DaemonError> {
        match job.job_status {
            JobStatus::OffchainProofRequested => {
                if let Err(e) = process_offchain_proof_stage(
                    job.clone(),
                    self.db_manager.clone(),
                    self.bankai.clone(),
                )
                .await
                {
                    self.handle_job_error(job.job_id).await?;
                    error!("Error processing offchain proof stage: {:?}", e);
                    return Err(e);
                }
            }
            JobStatus::WrapProofRequested => match job.job_type {
                JobType::SyncCommitteeUpdate => {
                    if let Err(e) = process_committee_wrapping_stage(
                        job.clone(),
                        self.db_manager.clone(),
                        self.bankai.clone(),
                    )
                    .await
                    {
                        self.handle_job_error(job.job_id).await?;
                        error!("Error processing committee wrapping stage: {:?}", e);
                        return Err(e);
                    }
                }
                JobType::EpochBatchUpdate => {
                    if let Err(e) = process_epoch_batch_wrapping_stage(
                        job.clone(),
                        self.db_manager.clone(),
                        self.bankai.clone(),
                    )
                    .await
                    {
                        self.handle_job_error(job.job_id).await?;
                        error!("Error processing epoch batch wrapping stage: {:?}", e);
                        return Err(e);
                    }
                }
            },
            JobStatus::OffchainComputationFinished => match job.job_type {
                JobType::EpochBatchUpdate => {
                    if let Err(e) = broadcast_epoch_batch(
                        job.clone(),
                        self.db_manager.clone(),
                        self.bankai.clone(),
                    )
                    .await
                    {
                        self.handle_job_error(job.job_id).await?;
                        error!("Error processing epoch bacth broadcast stage: {:?}", e);
                        return Err(e);
                    }
                }
                JobType::SyncCommitteeUpdate => {
                    if let Err(e) = broadcast_sync_committee(
                        job.clone(),
                        self.db_manager.clone(),
                        self.bankai.clone(),
                    )
                    .await
                    {
                        self.handle_job_error(job.job_id).await?;
                        error!("Error processing sync committee broadcast stage: {:?}", e);
                        return Err(e);
                    }
                }
            },
            _ => unimplemented!(),
        }

        Ok(())
    }

    pub async fn handle_job_error(&self, job_id: Uuid) -> Result<(), DaemonError> {
        let job_data = self.db_manager.get_job_by_id(job_id).await?.unwrap();
        self.db_manager
            .set_failure_info(job_id, job_data.job_status)
            .await?;
        self.db_manager
            .update_job_status(job_id, JobStatus::Error)
            .await?;
        Ok(())
    }
}

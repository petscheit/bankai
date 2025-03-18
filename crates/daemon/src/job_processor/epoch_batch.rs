use std::sync::Arc;

use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::{cairo_runner::{self, generate_committee_update_pie}, db::manager::DatabaseManager, types::{job::{AtlanticJobType, Job, JobStatus, JobType}, proofs::epoch_batch::EpochUpdateBatch, traits::{Exportable, ProofType}}, utils::{config, constants, helpers}, BankaiClient};
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use crate::error::DaemonError;


pub struct EpochBatchJobProcessor {
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
}

impl EpochBatchJobProcessor {
    pub fn new(db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Self {
        Self { db_manager, bankai }
    }

    pub async fn create_job(
        db_manager: Arc<DatabaseManager>,
        slot: u64,
        epoch_start: u64,
        epoch_end: u64,
    ) -> Result<Job, DaemonError> {
        let job_id = Uuid::new_v4();
        let job = Job {
            job_id: job_id.clone(),
            job_type: JobType::EpochBatchUpdate,
            job_status: JobStatus::Created,
            slot: Some(slot),
            batch_range_begin_epoch: Some(epoch_start),
            batch_range_end_epoch: Some(epoch_end),
        };

        match db_manager.create_job(job.clone()).await {
            Ok(()) => {
                info!(
                    "[EPOCH BATCH UPDATE][{}] Job created successfully. Epochs range from {} to {} | Sync committee involved: {}",
                    job_id, epoch_start, epoch_end, helpers::get_sync_committee_id_by_epoch(epoch_end)
                );
      
                Ok(job)
            }
            Err(e) => return Err(e.into())
        }
    }

    pub async fn process_job(
        &self,
        job: Job,
    ) -> Result<(), DaemonError> {
        info!(
            "[BATCH EPOCH JOB][{}] Preparing inputs for program for epochs from {} to {}...", 
            job.job_id, job.batch_range_begin_epoch.unwrap(), job.batch_range_end_epoch.unwrap()
        );
        
        let circuit_inputs = EpochUpdateBatch::new_by_epoch_range(
            &self.bankai,
            self.db_manager.clone(),
            job.batch_range_begin_epoch.unwrap(),
            job.batch_range_end_epoch.unwrap(),
            job.job_id,
        ).await.map_err(|e| bankai_core::types::proofs::ProofError::EpochBatch(e))?;

        let name = circuit_inputs.name();

        let input_path = circuit_inputs.export()?;
        info!(
            "[BATCH EPOCH JOB][{}] Circuit inputs saved at {:?}",
            job.job_id, input_path
        );

        info!("[BATCH EPOCH JOB][{}] Starting trace generation...", job.job_id);

        let pie = cairo_runner::generate_epoch_batch_pie(
            circuit_inputs,
            &self.bankai.config,
            Some(self.db_manager.clone()),
            Some(job.job_id),
        )
        .await?;


        self.db_manager
            .update_job_status(job.job_id, JobStatus::PieGenerated)
            .await?;

        info!("[BATCH EPOCH JOB][{}] Uploading PIE and sending proof generation request to Atlantic...", job.job_id);

        let batch_id = self.bankai.atlantic_client.submit_batch(pie, ProofType::EpochBatch, name).await?;

        info!(
            "[BATCH EPOCH JOB][{}] Proof generation batch submitted to Atlantic. QueryID: {}",
            job.job_id, batch_id
        );

        self.db_manager
            .update_job_status(job.job_id, JobStatus::OffchainProofRequested)
            .await?;

        self.db_manager
            .set_atlantic_job_queryid(
                job.job_id,
                batch_id.clone(),
                AtlanticJobType::ProofGeneration,
            )
            .await?;

        Ok(())
    }

}

use std::sync::Arc;

use bankai_core::{
    cairo_runner::{self},
    db::manager::DatabaseManager,
    types::{
        job::{AtlanticJobType, Job, JobStatus, JobType},
        proofs::epoch_batch::EpochUpdateBatch,
        traits::{Exportable, ProofType},
    },
    utils::helpers,
    BankaiClient,
};
use tracing::info;
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
            job_id,
            job_type: JobType::EpochBatchUpdate,
            job_status: JobStatus::Created,
            slot: Some(slot),
            batch_range_begin_epoch: Some(epoch_start),
            batch_range_end_epoch: Some(epoch_end),
        };

        match db_manager.create_job(job.clone()).await {
            Ok(()) => {
                info!(
                    job_id = %job_id,
                    job_type = %job.job_type,
                    epoch_start = epoch_start,
                    epoch_end = epoch_end,
                    sync_committee_id = helpers::get_sync_committee_id_by_epoch(epoch_end),
                    "Job created successfully"
                );

                Ok(job)
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn process_job(&self, job: Job) -> Result<(), DaemonError> {
        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            epoch_start = job.batch_range_begin_epoch.unwrap(),
            epoch_end = job.batch_range_end_epoch.unwrap(),
            "Preparing inputs for program for epochs"
        );

        let circuit_inputs = EpochUpdateBatch::new_by_epoch_range(
            &self.bankai,
            self.db_manager.clone(),
            job.batch_range_begin_epoch.unwrap(),
            job.batch_range_end_epoch.unwrap(),
            job.job_id,
        )
        .await
        .map_err(bankai_core::types::proofs::ProofError::EpochBatch)?;

        let name = circuit_inputs.name();

        let input_path = circuit_inputs.export()?;
        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            input_path = ?input_path,
            "Circuit inputs saved"
        );

        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            "Starting trace generation"
        );

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

        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            "Uploading PIE and sending proof generation request to Atlantic"
        );

        let batch_id = self
            .bankai
            .atlantic_client
            .submit_batch(pie, ProofType::EpochBatch, name)
            .await?;

        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            atlantic_query_id = %batch_id,
            "Proof generation batch submitted to Atlantic"
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

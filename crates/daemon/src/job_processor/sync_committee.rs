use bankai_core::{
    cairo_runner::generate_committee_update_pie,
    db::manager::DatabaseManager,
    types::{
        job::{AtlanticJobType, Job, JobStatus, JobType},
        traits::{Exportable, ProofType},
    },
    utils::{constants, helpers},
    BankaiClient,
};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::error::DaemonError;

pub struct SyncCommitteeJobProcessor {
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
}

impl SyncCommitteeJobProcessor {
    pub fn new(db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Self {
        Self { db_manager, bankai }
    }

    pub async fn create_job_from_event(
        db_manager: Arc<DatabaseManager>,
        latest_verified_sync_committee_id: u64,
        latest_verified_epoch_slot: u64,
    ) -> Result<Option<Job>, DaemonError> {
        let lowest_required_committee_update_slot =
            latest_verified_sync_committee_id * constants::SLOTS_PER_SYNC_COMMITTEE;

        // Only proceed if we're at or past the required slot
        if latest_verified_epoch_slot < lowest_required_committee_update_slot {
            return Ok(None);
        }

        // The new sync committee is always included in the previous epoch when we decommit it, so we need to increment by 1 here
        let potential_new_committee_id =
            helpers::get_sync_committee_id_by_slot(latest_verified_epoch_slot) + 1;

        // Get latest committee progress information
        let last_sync_committee_in_progress = db_manager
            .get_latest_sync_committee_in_progress()
            .await?
            .unwrap_or(0);

        let last_done_sync_committee = db_manager
            .get_latest_done_sync_committee()
            .await?
            .unwrap_or(0);

        if potential_new_committee_id > last_sync_committee_in_progress
            && potential_new_committee_id > last_done_sync_committee
        {
            let job_id = Uuid::new_v4();
            let job = Job {
                job_id,
                job_type: JobType::SyncCommitteeUpdate,
                job_status: JobStatus::Created,
                slot: Some(latest_verified_epoch_slot),
                batch_range_begin_epoch: None,
                batch_range_end_epoch: None,
            };

            match db_manager.create_job(job.clone()).await {
                Ok(()) => return Ok(Some(job)),
                Err(e) => return Err(e.into()),
            }
        }
        Ok(None)
    }

    pub async fn process_job(&self, job: Job) -> Result<(), DaemonError> {
        if let Some(slot) = job.slot {
            let update_committee_id = helpers::get_sync_committee_id_by_slot(slot);
            let update = self.bankai.get_sync_committee_update(slot).await?;

            let name = update.name();

            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                committee_id = update_committee_id,
                "Sync committee update program inputs generated"
            );

            let input_path = update.export()?;
            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                input_path = ?input_path,
                "Circuit inputs saved"
            );

            self.db_manager
                .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                committee_id = update_committee_id,
                "Starting Cairo execution and PIE generation"
            );

            let pie =
                generate_committee_update_pie(update, &self.bankai.config, None, None).await?;

            self.db_manager
                .update_job_status(job.job_id, JobStatus::PieGenerated)
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                committee_id = update_committee_id,
                "PIE generated successfully"
            );

            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                committee_id = update_committee_id,
                "Sending committee update proof generation query to Atlantic"
            );

            let batch_id = self
                .bankai
                .atlantic_client
                .submit_batch(pie, ProofType::SyncCommittee, name)
                .await?;

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

            info!(
                job_id = %job.job_id,
                job_type = "SYNC_COMMITTEE_JOB",
                atlantic_query_id = %batch_id,
                "Proof generation batch submitted to Atlantic"
            );
        }

        Ok(())
    }
}

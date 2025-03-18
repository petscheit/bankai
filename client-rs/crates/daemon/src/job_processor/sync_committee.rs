use std::sync::Arc;

use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::{cairo_runner::generate_committee_update_pie, db::manager::DatabaseManager, types::{job::{AtlanticJobType, Job, JobStatus, JobType}, traits::{Exportable, ProofType}}, utils::{config, constants, helpers}, BankaiClient};
use tokio::sync::mpsc;
use tracing::{error, info};
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
        event: &HeadEvent,
        latest_verified_sync_committee_id: u64,
        latest_verified_epoch_slot: u64,
    ) -> Result<Option<Job>, DaemonError> {
        let lowest_required_committee_update_slot =
            latest_verified_sync_committee_id * constants::SLOTS_PER_SYNC_COMMITTEE;

        // Only proceed if we're at or past the required slot
        if latest_verified_epoch_slot <= lowest_required_committee_update_slot {
            return Ok(None);
        }

        // Get latest committee progress information
        let last_sync_committee_in_progress = db_manager
            .get_latest_sync_committee_in_progress()
            .await?
            .unwrap_or(0);
            
        let last_done_sync_committee = db_manager
            .get_latest_done_sync_committee()
            .await?
            .unwrap_or(0);

        println!("latest_verified_sync_committee_id: {}", latest_verified_sync_committee_id);
        println!("last_sync_committee_in_progress: {}", last_sync_committee_in_progress);
        println!("last_done_sync_committee: {}", last_done_sync_committee);

        // Only create a new job if:
        // 1. The latest verified committee is newer than what's in progress
        // 2. The latest verified committee is newer than what's already done
        if latest_verified_sync_committee_id > last_sync_committee_in_progress &&
        latest_verified_sync_committee_id > last_done_sync_committee {
            let job_id = Uuid::new_v4();
            let job = Job {
                job_id: job_id.clone(),
                job_type: JobType::SyncCommitteeUpdate,
                job_status: JobStatus::Created,
                slot: Some(latest_verified_epoch_slot),
                batch_range_begin_epoch: None,
                batch_range_end_epoch: None,
            };

            match db_manager.create_job(job.clone()).await {
                Ok(()) => return Ok(Some(job)),
                Err(e) => return Err(e.into())
            }
        }

        Ok(None)
    }

    pub async fn process_job(
        &self,
        job: Job,
    ) -> Result<(), DaemonError> {
        if let Some(slot) = job.slot {
            let update_committee_id = helpers::get_sync_committee_id_by_slot(slot);
            let update = self.bankai
            .get_sync_committee_update(slot)
            .await?;
            
            let name = update.name();

            info!(
                "[SYNC COMMITTEE JOB][{}] Sync committee update program inputs generated: {:?}",
                job.job_id, update_committee_id
            );

            let input_path = update.export()?;
            info!(
                "[SYNC COMMITTEE JOB][{}] Circuit inputs saved at {:?}",
                job.job_id, input_path
            );
                
            self.db_manager
                .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                .await?;

            info!(
                "[SYNC COMMITTEE JOB][{}] Starting Cairo execution and PIE generation for Sync Committee: {}...",
                job.job_id, update_committee_id
            );

            let pie = generate_committee_update_pie(update, &self.bankai.config, None, None).await?;

            self.db_manager
                .update_job_status(job.job_id, JobStatus::PieGenerated)
                .await?;

            info!(
                "[SYNC COMMITTEE JOB][{}] Pie generated successfully for Sync Committee: {}...",
                job.job_id, update_committee_id
            );

            info!("[SYNC COMMITTEE JOB][{}] Sending committee update proof generation query to Atlantic: {}", 
                job.job_id, update_committee_id);

            let batch_id = self.bankai
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
                "[SYNC COMMITTEE JOB][{}] Proof generation batch submitted to atlantic. QueryID: {}",
                job.job_id, batch_id
            );
        }
       
        Ok(())
    }
}

   
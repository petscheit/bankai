use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::utils::constants;
use bankai_core::utils::helpers;
use bankai_core::{db::manager::DatabaseManager, types::job::Job, BankaiClient};
use num_traits::ToPrimitive;
use std::cmp;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::error::DaemonError;
use crate::job_processor::epoch_batch::EpochBatchJobProcessor;
use crate::job_processor::sync_committee::SyncCommitteeJobProcessor;

pub(crate) async fn create_new_jobs(
    parsed_event: &HeadEvent,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
    tx: mpsc::Sender<Job>,
) -> Result<(), DaemonError> {
    // Fetch the latest verified epoch slot and sync committee id from smart contract
    let latest_verified_epoch_slot = bankai
        .starknet_client
        .get_latest_epoch_slot(&bankai.config)
        .await?
        .to_u64()
        .unwrap();

    let latest_verified_sync_committee_id = bankai
        .starknet_client
        .get_latest_committee_id(&bankai.config)
        .await?
        .to_u64()
        .unwrap();

    let sync_committee_update_job = SyncCommitteeJobProcessor::create_job_from_event(
        db_manager.clone(),
        latest_verified_sync_committee_id,
        latest_verified_epoch_slot,
    )
    .await?;

    if let Some(job) = sync_committee_update_job {
        tx.send(job).await?;
    }

    let jobs_in_progress = db_manager.count_jobs_in_progress().await?;
    if jobs_in_progress.unwrap() >= constants::MAX_CONCURRENT_JOBS_IN_PROGRESS {
        info!("Max concurrent jobs in progress limit reached, skipping epoch batch creation");
        return Ok(());
    }

    let latest_verified_epoch = helpers::slot_to_epoch_id(latest_verified_epoch_slot);
    let latest_scheduled_epoch = db_manager.get_latest_epoch_in_progress().await?.unwrap();
    let latest_epoch = cmp::max(latest_verified_epoch, latest_scheduled_epoch);
    let epoch_gap = helpers::slot_to_epoch_id(parsed_event.slot) - latest_epoch;
    if epoch_gap >= constants::TARGET_BATCH_SIZE {
        // we are in syncing mode and need to catch up
        let start_epoch = latest_epoch + 1;
        let current_sync_committee_id = helpers::get_sync_committee_id_by_epoch(start_epoch);

        // we need to make sure that each batch is completed within the same sync committee
        let last_epoch_in_sync_committee =
            helpers::get_last_epoch_for_sync_committee(current_sync_committee_id);

        let end_epoch = cmp::min(
            latest_epoch + constants::TARGET_BATCH_SIZE,
            last_epoch_in_sync_committee,
        );

        if db_manager.check_job_exists(start_epoch, end_epoch).await? {
            info!(
                "Job already exists for epoch from {} to {}, skipping",
                start_epoch, end_epoch
            );
            return Ok(());
        }

        let epoch_batch = EpochBatchJobProcessor::create_job(
            db_manager.clone(),
            parsed_event.slot,
            start_epoch,
            end_epoch,
        )
        .await?;
        tx.send(epoch_batch).await?;
    }

    Ok(())
}

use std::sync::Arc;

use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::{
    cairo_runner::generate_committee_update_pie,
    db::manager::{DatabaseManager, JobSchema},
    types::{
        job::{AtlanticJobType, Job, JobStatus, JobType},
        proofs::{epoch_batch::EpochUpdateBatch, sync_committee::SyncCommitteeUpdate},
        traits::{Exportable, ProofType},
    },
    utils::{config, constants, helpers},
    BankaiClient,
};
use num_traits::ToPrimitive;
use starknet::core::types::Felt;
use std::sync::OnceLock;
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info};
use uuid::Uuid;

// Add a static semaphore to limit concurrent submissions
static SUBMISSION_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

fn get_semaphore() -> &'static Semaphore {
    SUBMISSION_SEMAPHORE.get_or_init(|| Semaphore::new(1))
}

use crate::error::DaemonError;

pub async fn broadcast_epoch_batch(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
            job.batch_range_begin_epoch.unwrap(),
            job.batch_range_end_epoch.unwrap(),
        )
        .map_err(|e| bankai_core::types::proofs::ProofError::EpochBatch(e))?;

        let committee_slot = update.expected_circuit_outputs.latest_batch_output.slot;
        let required_sync_committee_id: Felt =
            helpers::get_sync_committee_id_by_slot(committee_slot).into();

        let latest_verified_committee_id = bankai
            .starknet_client
            .get_latest_committee_id(&bankai.config)
            .await?;

        if required_sync_committee_id > latest_verified_committee_id {
            info!(
                "[EPOCH BATCH JOB][{}] Waiting for sync committee update. Required: {}, Latest: {}",
                job.job_id, required_sync_committee_id, latest_verified_committee_id
            );
            return Ok(());
        }

        // Acquire the semaphore permit before submitting the update
        let _permit = get_semaphore()
            .acquire()
            .await
            .expect("Failed to acquire semaphore");
        info!(
            "[EPOCH BATCH JOB][{}] Acquired submission permit, proceeding with on-chain update",
            job.job_id
        );

        let tx_hash = bankai
            .starknet_client
            .submit_update(update.expected_circuit_outputs.clone(), &bankai.config)
            .await?;

        info!("[EPOCH BATCH JOB][{}] Successfully called epoch batch update onchain, transaction confirmed, txhash: {}", 
            job.job_id, tx_hash);

        db_manager.set_job_txhash(job.job_id, tx_hash).await?;

        bankai
            .starknet_client
            .wait_for_confirmation(tx_hash)
            .await?;

        // Permit is automatically released when _permit goes out of scope

        info!(
            "[EPOCH BATCH JOB][{}] Transaction is confirmed on-chain!",
            job.job_id
        );
        db_manager
            .update_job_status(job.job_id, JobStatus::Done)
            .await?;

        let batch_root = update.expected_circuit_outputs.batch_root;
        for (index, epoch) in update.circuit_inputs.epochs.iter().enumerate() {
            {
                info!(
                    "[EPOCH BATCH JOB][{}] Inserting epoch data to DB: Index in batch: {}: {:?}",
                    job.job_id, index, epoch.expected_circuit_outputs
                );
                db_manager
                    .insert_verified_epoch_decommitment_data(
                        helpers::slot_to_epoch_id(epoch.expected_circuit_outputs.slot), //index.to_u64().unwrap(),
                        epoch.expected_circuit_outputs.beacon_header_root,
                        epoch.expected_circuit_outputs.beacon_state_root,
                        epoch.expected_circuit_outputs.slot,
                        epoch.expected_circuit_outputs.committee_hash,
                        epoch.expected_circuit_outputs.n_signers,
                        epoch.expected_circuit_outputs.execution_header_hash,
                        epoch.expected_circuit_outputs.execution_header_height,
                        index,
                        batch_root,
                    )
                    .await?;
            }
        }
    }

    Ok(())
}

pub async fn broadcast_sync_committee(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        let slot = job_data.slot.to_u64().unwrap();
        let sync_committee_id = helpers::slot_to_sync_committee_id(slot);

        let update = SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(job.slot.unwrap())
            .map_err(|e| bankai_core::types::proofs::ProofError::SyncCommittee(e))?;

        // Acquire the semaphore permit before submitting the update
        let _permit = get_semaphore()
            .acquire()
            .await
            .expect("Failed to acquire semaphore");
        info!(
            "[SYNC COMMITTEE JOB][{}] Acquired submission permit, proceeding with on-chain update",
            job.job_id
        );

        let tx_hash = bankai
            .starknet_client
            .submit_update(update.expected_circuit_outputs, &bankai.config)
            .await?;

        info!("[SYNC COMMITTEE JOB][{}] Successfully called sync committee ID {} update onchain, transaction confirmed, txhash: {}", 
            job.job_id, sync_committee_id, tx_hash);

        db_manager.set_job_txhash(job.job_id, tx_hash).await?;

        bankai
            .starknet_client
            .wait_for_confirmation(tx_hash)
            .await?;

        // Permit is automatically released when _permit goes out of scope

        info!(
            "[SYNC COMMITTEE JOB][{}] Transaction is confirmed on-chain!",
            job.job_id
        );
        db_manager
            .update_job_status(job.job_id, JobStatus::OffchainProofRetrieved)
            .await?;

        // Insert data to DB after successful onchain sync committee verification
        //let sync_committee_hash = update.expected_circuit_outputs.committee_hash;
        let sync_committee_hash = match bankai
            .starknet_client
            .get_committee_hash(slot, &bankai.config)
            .await
        {
            Ok(sync_committee_hash) => sync_committee_hash,
            Err(e) => {
                // Handle the error
                return Err(e.into());
            }
        };

        let sync_committee_hash_str = sync_committee_hash
            .iter()
            .map(|felt| felt.to_hex_string())
            .collect::<Vec<_>>()
            .join("");

        db_manager
            .insert_verified_sync_committee(slot, sync_committee_hash_str)
            .await?;
        db_manager
            .update_job_status(job.job_id, JobStatus::Done)
            .await?;

        info!(
            "[SYNC COMMITTEE JOB][{}] Sync committee verified onchain, job is done",
            job.job_id
        );
    }

    Ok(())
}

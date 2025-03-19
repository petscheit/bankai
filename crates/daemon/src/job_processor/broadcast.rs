use std::sync::Arc;

use bankai_core::{
    db::manager::DatabaseManager,
    types::{
        job::{Job, JobStatus},
        proofs::{epoch_batch::EpochUpdateBatch, sync_committee::SyncCommitteeUpdate},
    },
    utils::helpers,
    BankaiClient,
};
use num_traits::ToPrimitive;
use starknet::core::types::Felt;
use std::sync::OnceLock;
use tokio::sync::Semaphore;
use tracing::info;

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
    let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
        job.batch_range_begin_epoch.unwrap(),
        job.batch_range_end_epoch.unwrap(),
    )
    .map_err(bankai_core::types::proofs::ProofError::EpochBatch)?;

    let committee_slot = update.expected_circuit_outputs.latest_batch_output.slot;
    let required_sync_committee_id: Felt =
        helpers::get_sync_committee_id_by_slot(committee_slot).into();

    let latest_verified_committee_id = bankai
        .starknet_client
        .get_latest_committee_id(&bankai.config)
        .await?;

    if required_sync_committee_id > latest_verified_committee_id {
        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            required_committee_id = %required_sync_committee_id,
            latest_committee_id = %latest_verified_committee_id,
            "Waiting for sync committee update"
        );
        return Ok(());
    }

    // Acquire the semaphore permit before submitting the update
    let _permit = get_semaphore()
        .acquire()
        .await
        .expect("Failed to acquire semaphore");
    
    info!(
        job_id = %job.job_id,
        job_type = %job.job_type,
        "Acquired submission permit, proceeding with on-chain update"
    );

    let tx_hash = bankai
        .starknet_client
        .submit_update(update.expected_circuit_outputs.clone(), &bankai.config)
        .await?;

    // Add a small delay after submission before checking status
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    info!(
        job_id = %job.job_id,
        job_type = %job.job_type,
        tx_hash = %tx_hash,
        "Successfully called epoch batch update onchain, transaction confirmed"
    );

    db_manager.set_job_txhash(job.job_id, tx_hash).await?;

    bankai
        .starknet_client
        .wait_for_confirmation(tx_hash)
        .await?;

    // Permit is automatically released when _permit goes out of scope

    info!(
        job_id = %job.job_id,
        job_type = %job.job_type,
        "Transaction is confirmed on-chain"
    );
    
    db_manager
        .update_job_status(job.job_id, JobStatus::Done)
        .await?;

    let batch_root = update.expected_circuit_outputs.batch_root;
    for (index, epoch) in update.circuit_inputs.epochs.iter().enumerate() {
        {
            info!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                batch_index = index,
                epoch_data = ?epoch.expected_circuit_outputs,
                "Inserting epoch data to DB"
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
            .map_err(bankai_core::types::proofs::ProofError::SyncCommittee)?;

        // Acquire the semaphore permit before submitting the update
        let _permit = get_semaphore()
            .acquire()
            .await
            .expect("Failed to acquire semaphore");
            
        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            "Acquired submission permit, proceeding with on-chain update"
        );

        let tx_hash = bankai
            .starknet_client
            .submit_update(update.expected_circuit_outputs, &bankai.config)
            .await?;

        info!(
            job_id = %job.job_id,
            job_type =  %job.job_type,
            committee_id = sync_committee_id,
            tx_hash = %tx_hash,
            "Successfully called sync committee update onchain, transaction confirmed"
        );

        db_manager.set_job_txhash(job.job_id, tx_hash).await?;

        bankai
            .starknet_client
            .wait_for_confirmation(tx_hash)
            .await?;

        // Permit is automatically released when _permit goes out of scope

        info!(
            job_id = %job.job_id,
            job_type =  %job.job_type,
            "Transaction is confirmed on-chain"
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
            job_id = %job.job_id,
            job_type =  %job.job_type,
            "Sync committee verified onchain, job is done"
        );
    }

    Ok(())
}

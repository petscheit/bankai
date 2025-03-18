use std::sync::Arc;

use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::{cairo_runner::generate_committee_update_pie, db::manager::{DatabaseManager, JobSchema}, types::{job::{AtlanticJobType, Job, JobStatus, JobType}, proofs::{epoch_batch::EpochUpdateBatch, sync_committee::SyncCommitteeUpdate}, traits::{Exportable, ProofType}}, utils::{config, constants, helpers}, BankaiClient};
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info};
use uuid::Uuid;
use num_traits::ToPrimitive;
use std::sync::OnceLock;

use crate::error::DaemonError;

pub async fn broadcast_epoch_batch(job: Job, db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;
    
    if let Some(job_data) = job_data {
            let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                job.batch_range_begin_epoch.unwrap(), 
                job.batch_range_end_epoch.unwrap()
            ).map_err(|e| bankai_core::types::proofs::ProofError::EpochBatch(e))?;

            let committee_slot = update.ex

            info!(
                "[EPOCH BATCH JOB][{}] Proof wrapping done by Atlantic. QueryID: {:?}",
                job.job_id, job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );

            // Acquire the semaphore permit before submitting the update
            let _permit = get_semaphore().acquire().await.expect("Failed to acquire semaphore");
            info!("[EPOCH BATCH JOB][{}] Acquired submission permit, proceeding with on-chain update", job.job_id);
            
            let tx_hash = bankai
                .starknet_client
                .submit_update(
                    update.expected_circuit_outputs.clone(),
                    &bankai.config,
                )
                .await?;
    
            info!("[EPOCH BATCH JOB][{}] Successfully called epoch batch update onchain, transaction confirmed, txhash: {}", 
                job.job_id, tx_hash);

            db_manager.set_job_txhash(job.job_id, tx_hash).await?;

            bankai.starknet_client.wait_for_confirmation(tx_hash).await?;
            
            // Permit is automatically released when _permit goes out of scope
            
            info!("[EPOCH BATCH JOB][{}] Transaction is confirmed on-chain!", job.job_id);
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
        } else {
            info!("[EPOCH BATCH JOB][{}] Proof wrapping not done by Atlantic yet. QueryID: {:?}",
                job.job_id, job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
        }
    }

    Ok(())
}
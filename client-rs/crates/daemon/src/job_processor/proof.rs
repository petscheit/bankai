use std::sync::Arc;

use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::{cairo_runner::generate_committee_update_pie, db::manager::{DatabaseManager, JobSchema}, types::{job::{AtlanticJobType, Job, JobStatus, JobType}, proofs::{epoch_batch::EpochUpdateBatch, sync_committee::SyncCommitteeUpdate}, traits::{Exportable, ProofType}}, utils::{config, constants, helpers}, BankaiClient};
use tokio::sync::{mpsc, Semaphore};
use tracing::{error, info};
use uuid::Uuid;
use num_traits::ToPrimitive;
use std::sync::OnceLock;

use crate::error::DaemonError;

// Add a static semaphore to limit concurrent submissions
static SUBMISSION_SEMAPHORE: OnceLock<Semaphore> = OnceLock::new();

fn get_semaphore() -> &'static Semaphore {
    SUBMISSION_SEMAPHORE.get_or_init(|| Semaphore::new(1))
}

pub async fn process_offchain_proof_stage(job: Job, db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        info!(
            "[OFFCHAIN PROOF JOB][{}] Waiting for completion of Atlantic job. QueryID: {:?}",
            job.job_id, job_data.atlantic_proof_generate_batch_id
        );

        let batch_id = job_data.atlantic_proof_generate_batch_id.unwrap();

        let status = bankai
            .atlantic_client
                .check_batch_status(&batch_id)
                .await?;

        if status == "DONE" {
            info!(
                "[OFFCHAIN PROOF JOB][{}] Proof generation done by Atlantic. QueryID: {}",
                job.job_id, batch_id
            );

            let proof = bankai
                .atlantic_client
                .fetch_proof(batch_id.as_str())
                .await?;

            info!(
                "[OFFCHAIN PROOF JOB][{}] Proof retrieved from Atlantic. QueryID: {}",
                job.job_id, batch_id
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainProofRetrieved)
                .await?;

            info!("[OFFCHAIN PROOF JOB][{}] Sending proof wrapping query to Atlantic..", job.job_id);
            let wrapping_batch_id = bankai.atlantic_client.submit_wrapped_proof(proof, config::BankaiConfig::default().cairo_verifier_path, batch_id).await?;
            info!(
                "[OFFCHAIN PROOF JOB][{}] Proof wrapping query submitted to Atlantic. Wrapping QueryID: {}",
                job.job_id, wrapping_batch_id
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::WrapProofRequested)
                .await?;
            db_manager
                .set_atlantic_job_queryid(
                    job.job_id,
                    wrapping_batch_id.clone(),
                    AtlanticJobType::ProofWrapping,
                )
                .await?;
        }
    }
    Ok(())
}

pub async fn process_committee_wrapping_stage(job: Job, db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        let slot = job_data.slot.to_u64().unwrap();
        info!(  
            "[SYNC COMMITTEE JOB][{}] Checking completion of Atlantic proof wrapping job. QueryID: {:?}",
            job.job_id, job_data.atlantic_proof_wrapper_batch_id
        );
    
        let sync_commite_id = helpers::slot_to_sync_committee_id(slot);
    
        let status = bankai
            .atlantic_client
                .check_batch_status(job_data.atlantic_proof_wrapper_batch_id.clone().unwrap().as_str())
                .await?;
    
        if status == "DONE" {
            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            let update = SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(job.slot.unwrap()).map_err(|e| bankai_core::types::proofs::ProofError::SyncCommittee(e))?;

            info!(
                "[SYNC COMMITTEE JOB][{}] Proof wrapping done by Atlantic. QueryID: {:?}",
                job.job_id, job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
    
            // Acquire the semaphore permit before submitting the update
            let _permit = get_semaphore().acquire().await.expect("Failed to acquire semaphore");
            info!("[SYNC COMMITTEE JOB][{}] Acquired submission permit, proceeding with on-chain update", job.job_id);
            
            let tx_hash = bankai
                .starknet_client
                .submit_update(
                    update.expected_circuit_outputs,
                    &bankai.config,
                )
                .await?;
    
            info!("[SYNC COMMITTEE JOB][{}] Successfully called sync committee ID {} update onchain, transaction confirmed, txhash: {}", 
                job.job_id, sync_commite_id, tx_hash);
    
            db_manager.set_job_txhash(job.job_id, tx_hash).await?;
    
            bankai.starknet_client.wait_for_confirmation(tx_hash).await?;
            
            // Permit is automatically released when _permit goes out of scope
            
            info!("[SYNC COMMITTEE JOB][{}] Transaction is confirmed on-chain!", job.job_id);
            db_manager
                .update_job_status(job.job_id, JobStatus::Done)
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
                .insert_verified_sync_committee(
                    slot,
                    sync_committee_hash_str,
                )
                .await?;
        } else {
            info!("[SYNC COMMITTEE JOB][{}] Proof wrapping not done by Atlantic yet. QueryID: {:?}",
                job.job_id, job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
        }
    }


    Ok(())
}

pub async fn process_epoch_batch_wrapping_stage(job: Job, db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;
    
    if let Some(job_data) = job_data {

        let status = bankai
            .atlantic_client
                .check_batch_status(job_data.atlantic_proof_wrapper_batch_id.clone().unwrap().as_str())
                .await?;
    
        if status == "DONE" {
            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                job.batch_range_begin_epoch.unwrap(), 
                job.batch_range_end_epoch.unwrap()
            ).map_err(|e| bankai_core::types::proofs::ProofError::EpochBatch(e))?;

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
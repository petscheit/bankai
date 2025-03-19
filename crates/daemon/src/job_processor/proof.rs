use std::sync::Arc;

use crate::error::DaemonError;
use bankai_core::{
    db::manager::DatabaseManager,
    types::job::{AtlanticJobType, Job, JobStatus},
    BankaiClient,
};
use tracing::{error, info};

pub async fn process_offchain_proof_stage(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        info!(
            "[OFFCHAIN PROOF JOB][{}] Waiting for completion of Atlantic job. QueryID: {:?}",
            job.job_id, job_data.atlantic_proof_generate_batch_id
        );

        let batch_id = job_data.atlantic_proof_generate_batch_id.unwrap();

        let status = bankai.atlantic_client.check_batch_status(&batch_id).await?;

        info!(
            "[OFFCHAIN PROOF JOB][{}] Atlantic job status: {}",
            job.job_id, status
        );
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

            info!(
                "[OFFCHAIN PROOF JOB][{}] Sending proof wrapping query to Atlantic..",
                job.job_id
            );
            let wrapping_batch_id = bankai
                .atlantic_client
                .submit_wrapped_proof(
                    proof,
                    bankai.config.cairo_verifier_path.clone(),
                    batch_id,
                )
                .await?;
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
        } else if status == "FAILED" {
            error!(
                "[OFFCHAIN PROOF JOB][{}] Proof wrapping failed by Atlantic. QueryID: {:?}",
                job.job_id, batch_id
            );
            return Err(DaemonError::OffchainProofFailed(job.job_id.to_string()));
        } else {
            info!(
                "[OFFCHAIN PROOF JOB][{}] Proof wrapping not done by Atlantic yet. QueryID: {:?}",
                job.job_id, batch_id
            );
        }
    }
    Ok(())
}

pub async fn process_committee_wrapping_stage(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        info!(
            "[SYNC COMMITTEE JOB][{}] Checking completion of Atlantic proof wrapping job. QueryID: {:?}",
            job.job_id, job_data.atlantic_proof_wrapper_batch_id
        );

        let status = bankai
            .atlantic_client
            .check_batch_status(
                job_data
                    .atlantic_proof_wrapper_batch_id
                    .clone()
                    .unwrap()
                    .as_str(),
            )
            .await?;

        if status == "DONE" {
            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            info!(
                "[SYNC COMMITTEE JOB][{}] Proof wrapping done by Atlantic. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                .await?;
        } else if status == "FAILED" {
            error!(
                "[SYNC COMMITTEE JOB][{}] Proof wrapping failed by Atlantic. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
            return Err(DaemonError::ProofWrappingFailed(job.job_id.to_string()));
        } else {
            info!(
                "[SYNC COMMITTEE JOB][{}] Proof wrapping not done by Atlantic yet. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
        }
    }

    Ok(())
}

pub async fn process_epoch_batch_wrapping_stage(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), DaemonError> {
    let job_data = db_manager.get_job_by_id(job.job_id).await?;

    if let Some(job_data) = job_data {
        let status = bankai
            .atlantic_client
            .check_batch_status(
                job_data
                    .atlantic_proof_wrapper_batch_id
                    .clone()
                    .unwrap()
                    .as_str(),
            )
            .await?;

        if status == "DONE" {
            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            info!(
                "[EPOCH BATCH JOB][{}] Proof wrapping done by Atlantic. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                .await?;
        } else if status == "FAILED" {
            error!(
                "[EPOCH BATCH JOB][{}] Proof wrapping failed by Atlantic. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
            return Err(DaemonError::ProofWrappingFailed(job.job_id.to_string()));
        } else {
            info!(
                "[EPOCH BATCH JOB][{}] Proof wrapping not done by Atlantic yet. QueryID: {:?}",
                job.job_id,
                job_data.atlantic_proof_wrapper_batch_id.unwrap()
            );
        }
    }

    Ok(())
}

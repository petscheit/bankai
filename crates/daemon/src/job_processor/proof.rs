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
            job_id = %job.job_id,
            job_type = "OFFCHAIN_PROOF_JOB",
            atlantic_query_id = ?job_data.atlantic_proof_generate_batch_id,
            "Waiting for completion of Atlantic job"
        );

        let batch_id = job_data.atlantic_proof_generate_batch_id.unwrap();

        let status = bankai.atlantic_client.check_batch_status(&batch_id).await?;

        info!(
            job_id = %job.job_id,
            job_type = "OFFCHAIN_PROOF_JOB",
            atlantic_query_id = %batch_id,
            status = %status,
            "Atlantic job status received"
        );

        if status == "DONE" {
            info!(
                job_id = %job.job_id,
                job_type = "OFFCHAIN_PROOF_JOB",
                atlantic_query_id = %batch_id,
                "Proof generation done by Atlantic"
            );

            let proof = bankai
                .atlantic_client
                .fetch_proof(batch_id.as_str())
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = "OFFCHAIN_PROOF_JOB",
                atlantic_query_id = %batch_id,
                "Proof retrieved from Atlantic"
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainProofRetrieved)
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = "WRAP_PROOF_JOB",
                "Sending proof wrapping query to Atlantic"
            );

            let wrapping_batch_id = bankai
                .atlantic_client
                .submit_wrapped_proof(
                    proof,
                    bankai.config.cairo_verifier_path.clone(),
                    batch_id.clone(),
                )
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = "WRAP_PROOF_JOB",
                atlantic_query_id = %batch_id,
                wrapping_query_id = %wrapping_batch_id,
                "Proof wrapping query submitted to Atlantic"
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
                job_id = %job.job_id,
                job_type = "OFFCHAIN_PROOF_JOB",
                atlantic_query_id = %batch_id,
                "Proof wrapping failed by Atlantic"
            );
            return Err(DaemonError::OffchainProofFailed(job.job_id.to_string()));
        } else {
            info!(
                job_id = %job.job_id,
                job_type = "OFFCHAIN_PROOF_JOB",
                atlantic_query_id = %batch_id,
                "Proof wrapping not done by Atlantic yet"
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
        let batch_id = job_data.atlantic_proof_wrapper_batch_id.clone().unwrap();

        info!(
            job_id = %job.job_id,
            job_type = %job.job_type,
            atlantic_query_id = %batch_id,
            "Checking completion of Atlantic proof wrapping job"
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
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping done by Atlantic"
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                .await?;
        } else if status == "FAILED" {
            error!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping failed by Atlantic"
            );
            return Err(DaemonError::ProofWrappingFailed(job.job_id.to_string()));
        } else {
            info!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping not done by Atlantic yet"
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
        let batch_id = job_data.atlantic_proof_wrapper_batch_id.clone().unwrap();

        let status = bankai
            .atlantic_client
            .check_batch_status(batch_id.as_str())
            .await?;

        if status == "DONE" {
            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            info!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping done by Atlantic"
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                .await?;
        } else if status == "FAILED" {
            error!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping failed by Atlantic"
            );
            return Err(DaemonError::ProofWrappingFailed(job.job_id.to_string()));
        } else {
            info!(
                job_id = %job.job_id,
                job_type = %job.job_type,
                atlantic_query_id = %batch_id,
                "Proof wrapping not done by Atlantic yet"
            );
        }
    }

    Ok(())
}

use std::sync::Arc;

use thiserror::Error;
use tokio::{
    sync::AcquireError,
    task::{self, JoinError},
};
use tracing::info;
use uuid::Uuid;

use crate::{
    db::manager::DatabaseManager,
    types::{job::JobStatus, proofs::ProofError, traits::{ProofType, Provable}},
    utils::config::BankaiConfig,
};

pub struct CairoRunner();

impl CairoRunner {
    pub async fn generate_pie(
        input: &impl Provable,
        config: &BankaiConfig,
        db_manager: Option<Arc<DatabaseManager>>,
        job_id: Option<Uuid>,
    ) -> Result<(), CairoRunnerError> {
        // Acquire a permit from the semaphore.
        // If all permits are in use we will wait until one is available.
        let _permit = config
            .pie_generation_semaphore
            .clone()
            .acquire_owned()
            .await?;

        match db_manager {
            None => {}
            Some(db) => {
                let _ = db
                    .update_job_status(job_id.unwrap(), JobStatus::StartedTraceGeneration)
                    .await;
            }
        }

        let input_path = input.export().map_err(|e| CairoRunnerError::Run(e.to_string()))?;

        let program_path = match input.proof_type() {
            ProofType::Epoch => config.epoch_circuit_path.clone(),
            ProofType::SyncCommittee => config.committee_circuit_path.clone(),
            ProofType::EpochBatch => config.epoch_batch_circuit_path.clone(),
        };

        let pie_path = input.pie_path();
        info!("Generating trace...");
        let start_time = std::time::Instant::now();

        // Offload the blocking command execution to a dedicated thread
        let output = task::spawn_blocking(move || {
            std::process::Command::new("../venv/bin/cairo-run")
                .arg("--program")
                .arg(&program_path)
                .arg("--program_input")
                .arg(&input_path)
                .arg("--cairo_pie_output")
                .arg(&pie_path)
                .arg("--layout=all_cairo")
                .output()
                .map_err(|e| CairoRunnerError::Run(format!("Failed to execute commands: {}", e)))
        })
        .await??;

        let duration = start_time.elapsed();

        if !output.status.success() {
            return Err(CairoRunnerError::Run(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        } else {
            info!("Trace generated successfully in {:.2?}!", duration);
        }

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum CairoRunnerError {
    #[error("Cairo run error: {0}")]
    Run(String),
    #[error("Semaphore error: {0}")]
    Semaphore(#[from] AcquireError),
    #[error("Join error: {0}")]
    Join(#[from] JoinError),
}

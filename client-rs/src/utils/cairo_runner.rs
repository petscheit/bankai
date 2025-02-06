use std::sync::Arc;

use crate::state::JobStatus;
use crate::traits::ProofType;
use crate::BankaiConfig;
use crate::{traits::Provable, Error};
use tokio::task;
use tokio::task::JoinError;
use tracing::info;
use uuid::Uuid;

use super::database_manager::DatabaseManager;

pub struct CairoRunner();

impl CairoRunner {
    pub async fn generate_pie(
        input: &impl Provable,
        config: &BankaiConfig,
        db_manager: Option<Arc<DatabaseManager>>,
        job_id: Option<Uuid>,
    ) -> Result<(), Error> {
        // Acquire a permit from the semaphore.
        // If all permits are in use we will wait until one is available.
        let _permit = config
            .pie_generation_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::CairoRunError(format!("Semaphore error: {}", e)))?;

        match db_manager {
            None => {}
            Some(db) => {
                let _ = db
                    .update_job_status(job_id.unwrap(), JobStatus::StartedTraceGeneration)
                    .await;
            }
        }

        let input_path = input.export()?;

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
                .map_err(|e| Error::CairoRunError(format!("Failed to execute commands: {}", e)))
        })
        .await
        .map_err(|join_err: JoinError| {
            Error::CairoRunError(format!("spawn_blocking failed: {}", join_err))
        })??;

        let duration = start_time.elapsed();

        if !output.status.success() {
            return Err(Error::CairoRunError(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ));
        } else {
            info!("Trace generated successfully in {:.2?}!", duration);
        }

        Ok(())
    }
}

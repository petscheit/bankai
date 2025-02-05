use crate::traits::ProofType;
use crate::BankaiConfig;
use crate::{traits::Provable, Error};
use tokio::task;
use tokio::task::JoinError;
use tracing::{debug, info};
use std::path::Path;
use std::env;

pub struct CairoRunner();

impl CairoRunner {
    pub async fn generate_pie(input: &impl Provable, config: &BankaiConfig) -> Result<(), Error> {
        // Print current working directory
        if let Ok(current_dir) = env::current_dir() {
            info!("Current working directory: {}", current_dir.display());
        } else {
            debug!("Unable to determine current working directory");
        }

        // Acquire a permit from the semaphore.
        // If all permits are in use we will wait until one is available.
        let _permit = config
            .pie_generation_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error::CairoRunError(format!("Semaphore error: {}", e)))?;

        let input_path = input.inputs_path();
        if !Path::new(&input_path).exists() {
            return Err(Error::CairoRunError(format!(
                "Input file not found at: {}",
                input_path
            )));
        }
        info!("Cairo Input path: {}", input_path);

        let program_path = match input.proof_type() {
            ProofType::Epoch => config.epoch_circuit_path.clone(),
            ProofType::SyncCommittee => config.committee_circuit_path.clone(),
            ProofType::EpochBatch => config.epoch_batch_circuit_path.clone(),
        };
        if !Path::new(&program_path).exists() {
            return Err(Error::CairoRunError(format!(
                "Cairo program not found at: {}",
                program_path
            )));
        }

        let pie_path = input.pie_path();
        // Check if the directory for pie_path exists
        if let Some(pie_dir) = Path::new(&pie_path).parent() {
            if !pie_dir.exists() {
                return Err(Error::CairoRunError(format!(
                    "PIE output directory does not exist: {}",
                    pie_dir.display()
                )));
            }
        }
        info!("Generating trace...");
        let start_time = std::time::Instant::now();

        // Offload the blocking command execution to a dedicated thread
        let output = task::spawn_blocking(move || {
            std::process::Command::new("cairo-run")
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

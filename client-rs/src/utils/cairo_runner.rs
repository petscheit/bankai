use crate::traits::ProofType;
use crate::BankaiConfig;
use crate::{traits::Provable, Error};
use tracing::info;

pub struct CairoRunner();

impl CairoRunner {
    pub fn generate_pie(input: &impl Provable, config: &BankaiConfig) -> Result<(), Error> {
        let input_path = input.export()?;

        let program_path = match input.proof_type() {
            ProofType::Epoch => config.epoch_circuit_path.clone(),
            ProofType::SyncCommittee => config.committee_circuit_path.clone(),
            ProofType::EpochBatch => config.epoch_batch_circuit_path.clone(),
        };

        let pie_path = input.pie_path();
        info!("Generating trace...");
        let start_time = std::time::Instant::now();

        // Execute cairo-run command
        let output = std::process::Command::new("../venv/bin/cairo-run")
            .arg("--program")
            .arg(program_path)
            .arg("--program_input")
            .arg(input_path)
            .arg("--cairo_pie_output")
            .arg(pie_path)
            .arg("--layout=all_cairo")
            .output()
            .map_err(|e| Error::CairoRunError(format!("Failed to execute commands: {}", e)))?;

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

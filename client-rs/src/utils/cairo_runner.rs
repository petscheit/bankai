use crate::{traits::Provable, Error};
use serde::Serialize;
use crate::traits::ProofType;
use crate::BankaiConfig;
pub struct AtlanticClient {
    endpoint: String,
    api_key: String,
    pub client: reqwest::Client,
}

impl AtlanticClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self { endpoint, api_key, client: reqwest::Client::new() }
    }

    
}

pub struct CairoRunner();

impl CairoRunner {
   pub fn generate_pie(input: impl Provable, config: &BankaiConfig) -> Result<(), Error> {
        let input_path = input.export()?;

        let program_path = match input.proof_type() {
            ProofType::Epoch => config.epoch_circuit_path.clone(),
            ProofType::SyncCommittee => config.committee_circuit_path.clone(),
        };

        let pie_path = input.pie_path();
        println!("Generating trace...");
        let start_time = std::time::Instant::now();

        // Execute cairo-run command
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "source ../venv/bin/activate && cairo-run --program {} --program_input {} --cairo_pie_output {} --layout=all_cairo",
                program_path,
                input_path,
                pie_path
            ))
            .output()
            .map_err(|e| Error::CairoRunError(format!("Failed to execute commands: {}", e)))?;

        let duration = start_time.elapsed();

        if !output.status.success() {
            return Err(Error::CairoRunError(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        } else {
            println!("Trace generated successfully in {:.2?}!", duration);
        }

        Ok(())
    }
}
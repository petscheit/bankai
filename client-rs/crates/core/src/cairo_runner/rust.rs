use std::sync::Arc;

use bankai_runner::committee_update::{CommitteeUpdateCircuit, CircuitInput, CircuitOutput};
use bankai_runner::types::{Uint256, UInt384, Uint256Bits32, Felt};
pub use bankai_runner::run_committee_update;
use cairo_vm::Felt252;
use num_bigint::BigUint;
use tracing::info;
use uuid::Uuid;
use crate::db::manager::DatabaseManager;
use crate::types::error::BankaiCoreError;
use crate::types::job::JobStatus;
use crate::types::proofs::sync_committee::SyncCommitteeUpdate;
use crate::utils::config::BankaiConfig;
use cairo_vm::vm::runners::cairo_pie::CairoPie;

pub async fn generate_pie(
    input: SyncCommitteeUpdate,
    config: &BankaiConfig,
    db_manager: Option<Arc<DatabaseManager>>,
    job_id: Option<Uuid>,
) -> Result<CairoPie, BankaiCoreError> {
    
    // Print the current working directory
    match std::env::current_dir() {
        Ok(path) => info!("Current working directory: {:?}", path),
        Err(e) => info!("Failed to get current working directory: {}", e),
    }
    
    let _permit = config
        .pie_generation_semaphore
        .clone()
        .acquire_owned()
        .await.unwrap();

    match db_manager {
        None => {}
        Some(db) => {
            let _ = db
            .update_job_status(job_id.unwrap(), JobStatus::StartedTraceGeneration)
            .await;
        }
    }
    info!("Generating trace...");
    let start_time = std::time::Instant::now();

    let pie = run_committee_update(config.committee_circuit_path.as_str(), input.into())?;
    let duration = start_time.elapsed();

    info!("Trace generated successfully in {:.2?}!", duration);
    
    Ok(pie)

}


impl Into<CommitteeUpdateCircuit> for SyncCommitteeUpdate {
    fn into(self) -> CommitteeUpdateCircuit {
        let branch = self.circuit_inputs.next_sync_committee_branch.iter().map(|b| Uint256Bits32(BigUint::from_bytes_be(b.as_slice()))).collect::<Vec<Uint256Bits32>>();
        let circuit_input = CircuitInput {
            beacon_slot: Felt(Felt252::from(self.circuit_inputs.beacon_slot)),
            next_sync_committee_branch: branch,
            next_aggregate_sync_committee: UInt384(BigUint::from_bytes_be(self.circuit_inputs.next_aggregate_sync_committee.as_slice())),
            committee_keys_root: Uint256Bits32(BigUint::from_bytes_be(self.circuit_inputs.committee_keys_root.as_slice())),
        };
        let circuit_output = CircuitOutput {
            state_root: Uint256(BigUint::from_bytes_be(self.expected_circuit_outputs.state_root.as_slice())),
            slot: Felt(Felt252::from(self.expected_circuit_outputs.slot)),
            committee_hash: Uint256(BigUint::from_bytes_be(self.expected_circuit_outputs.committee_hash.as_slice())),
        };
        CommitteeUpdateCircuit {
            circuit_inputs: circuit_input,
            expected_circuit_outputs: circuit_output,
        }
    }
}
use std::sync::Arc;

use bankai_runner::committee_update::{CommitteeUpdateCircuit, CircuitInput, CircuitOutput};
use bankai_runner::epoch_update::{BeaconHeaderCircuit, EpochCircuitInputs, EpochUpdateCircuit, ExecutionHeaderCircuitProof, ExecutionPayloadHeaderCircuit};
use bankai_runner::types::{Uint256, UInt384, Uint256Bits32, Felt, G1CircuitPoint, G2CircuitPoint};
pub use bankai_runner::run_committee_update;
use cairo_vm::Felt252;
use num_bigint::BigUint;
use tracing::info;
use uuid::Uuid;
use crate::db::manager::DatabaseManager;
use crate::types::error::BankaiCoreError;
use crate::types::job::JobStatus;
use crate::types::proofs::epoch_update::{EpochUpdate, G1Point, G2Point};
use crate::types::proofs::sync_committee::SyncCommitteeUpdate;
use crate::utils::config::BankaiConfig;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
// use bankai_runner::epoch_update::EpochUpdateCircuit;
use bankai_runner::run_epoch_update;

pub async fn generate_epoch_update_pie(
    input: EpochUpdate,
    config: &BankaiConfig,
    db_manager: Option<Arc<DatabaseManager>>,
    job_id: Option<Uuid>,
) -> Result<CairoPie, BankaiCoreError> {

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

    let pie = run_epoch_update(config.epoch_circuit_path.as_str(), input.into())?;
    let duration = start_time.elapsed();

    info!("Trace generated successfully in {:.2?}!", duration);
    
    Ok(pie)
}
    

pub async fn generate_pie(
    input: SyncCommitteeUpdate,
    config: &BankaiConfig,
    db_manager: Option<Arc<DatabaseManager>>,
    job_id: Option<Uuid>,
) -> Result<CairoPie, BankaiCoreError> {
    
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

impl Into<EpochUpdateCircuit> for EpochUpdate {
    fn into(self) -> EpochUpdateCircuit {
        let beacon_header = BeaconHeaderCircuit {
            slot: Uint256(BigUint::from(self.circuit_inputs.header.slot)),
            proposer_index: Uint256(BigUint::from(self.circuit_inputs.header.proposer_index)),
            parent_root: Uint256(BigUint::from_bytes_be(self.circuit_inputs.header.parent_root.as_slice())),
            state_root: Uint256(BigUint::from_bytes_be(self.circuit_inputs.header.state_root.as_slice())),
            body_root: Uint256(BigUint::from_bytes_be(self.circuit_inputs.header.body_root.as_slice())),
        };
        let execution_header_proof: ExecutionHeaderCircuitProof = ExecutionHeaderCircuitProof {
            root: Uint256(BigUint::from_bytes_be(self.circuit_inputs.execution_header_proof.root.as_slice())),
            path: self.circuit_inputs.execution_header_proof.path.iter().map(|p| Uint256Bits32(BigUint::from_bytes_be(p.as_slice()))).collect::<Vec<Uint256Bits32>>(),
            leaf: Uint256(BigUint::from_bytes_be(self.circuit_inputs.execution_header_proof.leaf.as_slice())),
            index: Felt(Felt252::from(self.circuit_inputs.execution_header_proof.index)),
            execution_payload_header: ExecutionPayloadHeaderCircuit(self.circuit_inputs.execution_header_proof.execution_payload_header).to_field_roots(),
        };
        let inputs = EpochCircuitInputs {
            header: beacon_header,
            signature_point: self.circuit_inputs.signature_point.into(),
            aggregate_pub: self.circuit_inputs.aggregate_pub.into(),
            non_signers: self.circuit_inputs.non_signers.iter().map(|n| n.clone().into()).collect::<Vec<G1CircuitPoint>>(),
            execution_header_proof: execution_header_proof,
        };
        EpochUpdateCircuit {
            circuit_inputs: inputs,
        }
    }
}

impl Into<G1CircuitPoint> for G1Point {
    fn into(self) -> G1CircuitPoint {
        let json = serde_json::to_string(&self).unwrap();
        let parsed: G1CircuitPoint = serde_json::from_str(&json).unwrap();
        parsed
    }
}

impl Into<G2CircuitPoint> for G2Point {
    fn into(self) -> G2CircuitPoint {
        let json = serde_json::to_string(&self).unwrap();
        let parsed: G2CircuitPoint = serde_json::from_str(&json).unwrap();
        parsed
    }
}

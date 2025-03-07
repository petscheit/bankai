use std::collections::HashMap;

use crate::{hint_processor::CustomHintProcessor, types::{Bytes32, Felt, G1CircuitPoint, G2CircuitPoint, UInt384, Uint256, Uint256Bits32}};
use cairo_vm::{hint_processor::builtin_hint_processor::{builtin_hint_processor_definition::HintProcessorData, hint_utils::{get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name}}, types::exec_scope::ExecutionScopes, vm::{errors::hint_errors::HintError, vm_core::VirtualMachine}, Felt252};
use garaga_zero_hints::types::CairoType;
use num_bigint::BigUint;
use serde::Deserialize;
use beacon_types::{BeaconBlockBody, ExecPayload, ExecutionPayloadHeader, ExecutionPayloadHeaderBellatrix, ExecutionPayloadHeaderCapella, ExecutionPayloadHeaderDeneb, ExecutionPayloadHeaderElectra, MainnetEthSpec};
use serde_json;
use beacon_types::TreeHash;

#[derive(Debug, Deserialize)]
pub struct EpochUpdateCircuit {
    pub circuit_inputs: EpochCircuitInputs,
    // pub expected_circuit_outputs: ExpectedEpochUpdateOutputs,
}

#[derive(Debug, Deserialize)]
pub struct EpochCircuitInputs {
    pub header: BeaconHeaderCircuit,
    pub signature_point: G2CircuitPoint,
    pub aggregate_pub: G1CircuitPoint,
    pub non_signers: Vec<G1CircuitPoint>,
    pub execution_header_proof: ExecutionHeaderCircuitProof,
}

#[derive(Debug, Deserialize)]
pub struct ExecutionHeaderCircuitProof {
    pub root: Uint256,
    pub path: Vec<Uint256Bits32>,
    pub leaf: Uint256,
    pub index: Felt,
    pub execution_payload_header: Vec<Bytes32>,
}


#[derive(Debug, Deserialize)]
pub struct BeaconHeaderCircuit {
    pub slot: Uint256,
    pub proposer_index: Uint256,
    pub parent_root: Uint256,
    pub state_root: Uint256,
    pub body_root: Uint256,
}


pub struct ExecutionPayloadHeaderCircuit(pub ExecutionPayloadHeader<MainnetEthSpec>);

impl ExecutionPayloadHeaderCircuit {
    pub fn to_field_roots(&self) -> Vec<Bytes32> {
        // Helper function to convert any value to a padded 32-byte Uint256
        fn to_uint256<T: AsRef<[u8]>>(bytes: T) -> Bytes32 {
            let mut padded = vec![0; 32];
            let bytes = bytes.as_ref();
            // Copy bytes to the beginning of the padded array (right padding with zeros)
            padded[..bytes.len()].copy_from_slice(bytes);
            Bytes32::new(padded)
        }

        // Convert u64 to padded bytes
        fn u64_to_uint256(value: u64) -> Bytes32 {
            Bytes32::from_u64(value)
        }

        macro_rules! extract_common_fields {
            ($h:expr) => {
                vec![
                    to_uint256($h.parent_hash.0.as_bytes()),
                    to_uint256($h.fee_recipient.0.to_vec()),
                    to_uint256($h.state_root.0.to_vec()),
                    to_uint256($h.receipts_root.0.to_vec()),
                    to_uint256($h.logs_bloom.tree_hash_root().as_bytes()),
                    to_uint256($h.prev_randao.0.to_vec()),
                    u64_to_uint256($h.block_number),
                    u64_to_uint256($h.gas_limit),
                    u64_to_uint256($h.gas_used),
                    u64_to_uint256($h.timestamp),
                    to_uint256($h.extra_data.tree_hash_root().as_bytes()),
                    to_uint256($h.base_fee_per_gas.tree_hash_root().as_bytes()),
                    to_uint256($h.block_hash.0.as_bytes()),
                    to_uint256($h.transactions_root.as_bytes()),
                ]
            };
        }

        let roots = match &self.0 {
            ExecutionPayloadHeader::Bellatrix(h) => extract_common_fields!(h),
            ExecutionPayloadHeader::Capella(h) => {
                println!("Capella");
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_bytes()));
                roots
            },
            ExecutionPayloadHeader::Deneb(h) => {
                println!("Deneb");
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_bytes()));
                roots.push(u64_to_uint256(h.blob_gas_used));
                roots.push(u64_to_uint256(h.excess_blob_gas));
                println!("Deneb roots: {:?}", roots);
                for root in roots.iter() {
                    println!("root: {:?}", hex::encode(root.as_bytes()));
                }
                println!("length: {:?}", roots.len());
                roots
            },
            ExecutionPayloadHeader::Electra(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_bytes()));
                roots.push(u64_to_uint256(h.blob_gas_used));
                roots.push(u64_to_uint256(h.excess_blob_gas));
                // roots.push(to_uint256(h.deposit_requests_root.as_bytes()));
                // roots.push(to_uint256(h.withdrawal_requests_root.as_bytes()));
                roots
            },
        };

        roots
    }
}

pub const HINT_WRITE_EPOCH_INPUTS: &str = r#"write_epoch_inputs()"#;
// pub const HINT_ASSERT_RESULT: &str = r#"assert_result()"#;

impl CustomHintProcessor {

    pub fn write_epoch_inputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        println!("Writing epoch inputs");
        if let Some(epoch_update) = &self.epoch_input {
            let sig_point_ptr = get_relocatable_from_var_name("sig_point", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            println!("Sig point ptr: {:?}", sig_point_ptr);
            epoch_update.circuit_inputs.signature_point.to_memory(vm, sig_point_ptr)?;

            let mut header_ptr = get_relocatable_from_var_name("header", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            header_ptr = epoch_update.circuit_inputs.header.slot.to_memory(vm, header_ptr)?;
            header_ptr = epoch_update.circuit_inputs.header.proposer_index.to_memory(vm, header_ptr)?;
            header_ptr = epoch_update.circuit_inputs.header.parent_root.to_memory(vm, header_ptr)?;
            header_ptr = epoch_update.circuit_inputs.header.state_root.to_memory(vm, header_ptr)?;
            epoch_update.circuit_inputs.header.body_root.to_memory(vm, header_ptr)?;

            let mut signer_data_ptr = get_relocatable_from_var_name("signer_data", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            
            signer_data_ptr = epoch_update.circuit_inputs.aggregate_pub.to_memory(vm, signer_data_ptr)?;
            let non_signers_segment = vm.add_memory_segment();
            vm.insert_value(signer_data_ptr, &non_signers_segment)?;

            let mut segment_ptr = non_signers_segment;
            for i in 0..epoch_update.circuit_inputs.non_signers.len() {
                segment_ptr = epoch_update.circuit_inputs.non_signers[i].to_memory(vm, segment_ptr)?;
            }
            vm.insert_value((signer_data_ptr + 1)?, &Felt252::from(epoch_update.circuit_inputs.non_signers.len()))?;

            println!("ADDED SIGNER DATA: {:?}", signer_data_ptr);

            let mut execution_header_proof_ptr = get_relocatable_from_var_name("execution_header_proof", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            execution_header_proof_ptr = epoch_update.circuit_inputs.execution_header_proof.root.to_memory(vm, execution_header_proof_ptr)?;
            println!("wrote root");
            let path_segment = vm.add_memory_segment();
            vm.insert_value(execution_header_proof_ptr, &path_segment)?;
            execution_header_proof_ptr = (execution_header_proof_ptr + 1)?;
            println!("wrote path segment");
            let mut path_ptr = path_segment;
            for i in 0..epoch_update.circuit_inputs.execution_header_proof.path.len() {
                path_ptr = epoch_update.circuit_inputs.execution_header_proof.path[i].to_memory(vm, path_ptr)?;
            }
            println!("wrote path");
            execution_header_proof_ptr = epoch_update.circuit_inputs.execution_header_proof.leaf.to_memory(vm, execution_header_proof_ptr)?;
            println!("wrote leaf");
            execution_header_proof_ptr = epoch_update.circuit_inputs.execution_header_proof.index.to_memory(vm, execution_header_proof_ptr)?;
            println!("wrote index");
            let payload_fields_segment = vm.add_memory_segment();
            vm.insert_value(execution_header_proof_ptr, &payload_fields_segment)?;
            println!("wrote payload fields segment");
            let mut payload_fields_ptr = payload_fields_segment;
            for i in 0..epoch_update.circuit_inputs.execution_header_proof.execution_payload_header.len() {
                payload_fields_ptr = epoch_update.circuit_inputs.execution_header_proof.execution_payload_header[i].to_memory(vm, payload_fields_ptr)?;
            }
            println!("wrote payload fields");
            Ok(())
        } else {
            panic!("Committee input not found");
        }
    }

}

pub const HINT_CHECK_FORK_VERSION: &str = r#"check_fork_version()"#;

pub fn hint_check_fork_version(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    hint_data: &HintProcessorData,
    constants: &HashMap<String, Felt252>,
 ) -> Result<(), HintError> {

    let altair_activation_slot = constants.get("cairo.src.domain.Domain.ALTAIR_ACTIVATION_SLOT").unwrap();
    let bellatrix_activation_slot = constants.get("cairo.src.domain.Domain.BELLATRIX_ACTIVATION_SLOT").unwrap();
    let capella_activation_slot = constants.get("cairo.src.domain.Domain.CAPPELLA_ACTIVATION_SLOT").unwrap();
    let deneb_activation_slot = constants.get("cairo.src.domain.Domain.DENEB_ACTIVATION_SLOT").unwrap();
    
    let slot = &get_integer_from_var_name("slot", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
    // Determine the fork version based on the slot
    let fork_value = if slot < altair_activation_slot {
        Felt252::from(0)
    } else if slot < bellatrix_activation_slot {
        Felt252::from(1)
    } else if slot < capella_activation_slot {
        Felt252::from(2)
    } else if slot < deneb_activation_slot {
        Felt252::from(3)
    } else {
        Felt252::from(4)
    };
    
    // Store the fork value in the Cairo program
    let fork = get_relocatable_from_var_name("fork", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
    vm.insert_value(fork, &fork_value)?;
    
    Ok(())
}
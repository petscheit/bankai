use std::collections::HashMap;

use crate::{hint_processor::CustomHintProcessor, types::{Bytes32, Felt, G1CircuitPoint, G2CircuitPoint, Uint256, Uint256Bits32}};
use cairo_vm::{hint_processor::builtin_hint_processor::{builtin_hint_processor_definition::HintProcessorData, hint_utils::{get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name}}, types::{exec_scope::ExecutionScopes, relocatable::Relocatable}, vm::{errors::hint_errors::HintError, vm_core::VirtualMachine}, Felt252};
use garaga_zero_hints::types::CairoType;
use serde::Deserialize;
use beacon_types::{ExecutionPayloadHeader, MainnetEthSpec};

use beacon_types::TreeHash;

#[derive(Debug, Deserialize)]
pub struct EpochUpdateCircuit {
    pub circuit_inputs: EpochCircuitInputs,
    pub expected_circuit_outputs: ExpectedEpochUpdateCircuitOutputs,
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
                    to_uint256($h.parent_hash.0.as_slice()),
                    to_uint256($h.fee_recipient.0.to_vec()),
                    to_uint256($h.state_root.0.to_vec()),
                    to_uint256($h.receipts_root.0.to_vec()),
                    to_uint256($h.logs_bloom.tree_hash_root().as_slice()),
                    to_uint256($h.prev_randao.0.to_vec()),
                    u64_to_uint256($h.block_number),
                    u64_to_uint256($h.gas_limit),
                    u64_to_uint256($h.gas_used),
                    u64_to_uint256($h.timestamp),
                    to_uint256($h.extra_data.tree_hash_root().as_slice()),
                    to_uint256($h.base_fee_per_gas.tree_hash_root().as_slice()),
                    to_uint256($h.block_hash.0.as_slice()),
                    to_uint256($h.transactions_root.as_slice()),
                ]
            };
        }

        let roots = match &self.0 {
            ExecutionPayloadHeader::Bellatrix(h) => extract_common_fields!(h),
            ExecutionPayloadHeader::Capella(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_slice()));
                roots
            },
            ExecutionPayloadHeader::Deneb(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_slice()));
                roots.push(u64_to_uint256(h.blob_gas_used));
                roots.push(u64_to_uint256(h.excess_blob_gas));
                roots
            },
            ExecutionPayloadHeader::Electra(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_slice()));
                roots.push(u64_to_uint256(h.blob_gas_used));
                roots.push(u64_to_uint256(h.excess_blob_gas));
                // roots.push(to_uint256(h.deposit_requests_root.as_slice()));
                // roots.push(to_uint256(h.withdrawal_requests_root.as_slice()));
                roots
            },
        };

        roots
    }
}

pub const HINT_WRITE_EPOCH_UPDATE_INPUTS: &str = r#"write_epoch_update_inputs()"#;
pub const HINT_ASSERT_EPOCH_UPDATE_RESULT: &str = r#"verify_epoch_update_outputs()"#;

impl CustomHintProcessor {

    pub fn write_epoch_update_inputs(   
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(epoch_update) = &self.epoch_input {
            let epoch_update_ptr = get_relocatable_from_var_name("epoch_update", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            write_epoch_update(epoch_update_ptr, &epoch_update.circuit_inputs, vm)?;
            Ok(())
        } else {
            panic!("EpochUpdate input not found");
        }
    }

    pub fn assert_epoch_update_result(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let expected_outputs = &self.epoch_input.as_ref().expect("EpochUpdate input not found").expected_circuit_outputs;

        // Get the output pointer
        let output_ptr = (get_ptr_from_var_name("output_ptr", vm, &hint_data.ids_data, &hint_data.ap_tracking)? - 11)?;
        
        assert_epoch_update_result(vm, output_ptr, expected_outputs)
    }
}

pub const HINT_CHECK_FORK_VERSION: &str = r#"check_fork_version()"#;

pub fn hint_check_fork_version(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    hint_data: &HintProcessorData,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let slot = get_integer_from_var_name("slot", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
    let network_id: usize = get_integer_from_var_name("network_id", vm, &hint_data.ids_data, &hint_data.ap_tracking)?.try_into().unwrap();

    // Get the fork_data label address from Cairo memory
    let fork_schedule_ptr = get_ptr_from_var_name("fork_schedule", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
    
    // Each network has 12 values (6 forks × 2 values per fork)
    // For each fork: [version, slot]
    let network_offset = network_id * 12;

    // Read activation slots for the selected network
    let mut activation_slots = Vec::new();
    for i in 0..6 {
        let slot_address = (fork_schedule_ptr + (i * 2 + 1 + network_offset))?;
        let activation_slot = *vm.get_integer(slot_address)?;
        activation_slots.push(activation_slot);
    }
    
    let mut latest_fork = 0;
    for (i, activation_slot) in activation_slots.iter().enumerate() {
        if slot >= *activation_slot {
            latest_fork = i;
        }
    }   
    
    // Store the fork value in the Cairo program
    let fork = get_relocatable_from_var_name("fork", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
    vm.insert_value(fork, &Felt252::from(latest_fork))?;
    
    Ok(())
}


pub fn print_address_range(vm: &VirtualMachine, address: Relocatable, depth: usize, padding: Option<usize>) {
    let padding = padding.unwrap_or(0); // Default to 20 if not specified
    let start_offset = if address.offset >= padding { address.offset - padding } else { 0 };
    let end_offset = address.offset + depth + padding;

    println!("\nFull memory segment range for segment {}:", address.segment_index);
    println!("----------------------------------------");
    for i in start_offset..end_offset {
        let addr = Relocatable {
            segment_index: address.segment_index,
            offset: i,
        };
        match vm.get_maybe(&addr) {
            Some(value) => println!("Offset {}: {:?}", i, value),
            None => println!("Offset {}: <empty>", i),
        }
    }
    println!("----------------------------------------\n");
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExpectedEpochUpdateCircuitOutputs {
    pub beacon_header_root: Uint256,
    pub beacon_state_root: Uint256,
    pub committee_hash: Uint256,
    pub n_signers: Felt,
    pub slot: Felt,
    pub execution_header_hash: Uint256,
    pub execution_header_height: Felt,
}

pub fn write_epoch_update(
    epoch_update_ptr: Relocatable,
    circuit_inputs: &EpochCircuitInputs,
    vm: &mut VirtualMachine,
) -> Result<Relocatable, HintError> {    
    let mut current_ptr = epoch_update_ptr;
    
    // Write signature point
    current_ptr = circuit_inputs.signature_point.to_memory(vm, current_ptr)?;

    // Write header fields
    current_ptr = write_header_fields(vm, current_ptr, &circuit_inputs.header)?;
    
    // Write signer data (aggregate pub key and non-signers)
    current_ptr = write_signer_data(vm, current_ptr, circuit_inputs)?;

    // Write execution header proof
    current_ptr = write_execution_header_proof(vm, current_ptr, &circuit_inputs.execution_header_proof)?;
    
    Ok(current_ptr)
}

fn write_header_fields(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    header: &BeaconHeaderCircuit,
) -> Result<Relocatable, HintError> {
    ptr = header.slot.to_memory(vm, ptr)?;
    ptr = header.proposer_index.to_memory(vm, ptr)?;
    ptr = header.parent_root.to_memory(vm, ptr)?;
    ptr = header.state_root.to_memory(vm, ptr)?;
    ptr = header.body_root.to_memory(vm, ptr)?;
    Ok(ptr)
}

fn write_signer_data(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    circuit_inputs: &EpochCircuitInputs,
) -> Result<Relocatable, HintError> {    
    // Write aggregate public key
    ptr = circuit_inputs.aggregate_pub.to_memory(vm, ptr)?;
    
    // Create segment for non-signers and store its pointer
    let non_signers_segment = vm.add_memory_segment();
    vm.insert_value(ptr, &non_signers_segment)?;

    // Write all non-signers to the segment
    let mut segment_ptr = non_signers_segment;
    for non_signer in &circuit_inputs.non_signers {
        segment_ptr = non_signer.to_memory(vm, segment_ptr)?;
    }
    
    // Store the length of non-signers
    vm.insert_value((ptr + 1)?, &Felt252::from(circuit_inputs.non_signers.len()))?;
    
    Ok((ptr + 2)?)
}

fn write_execution_header_proof(
    vm: &mut VirtualMachine,
    mut ptr: Relocatable,
    proof: &ExecutionHeaderCircuitProof,
) -> Result<Relocatable, HintError> {
    
    // Write root
    ptr = proof.root.to_memory(vm, ptr)?;
    
    // Create and write path segment
    let path_segment = vm.add_memory_segment();
    vm.insert_value(ptr, &path_segment)?;
    ptr = (ptr + 1)?;
    
    // Write each path element
    let mut path_ptr = path_segment;
    for path_element in &proof.path {
        let element_segment = vm.add_memory_segment();
        vm.insert_value(path_ptr, &element_segment)?;
        path_ptr = (path_ptr + 1)?;
        
        path_element.to_memory(vm, element_segment)?;
    }
    
    // Write leaf and index
    ptr = proof.leaf.to_memory(vm, ptr)?;
    ptr = proof.index.to_memory(vm, ptr)?;
    
    // Create and write payload fields segment
    let payload_fields_segment = vm.add_memory_segment();
    vm.insert_value(ptr, &payload_fields_segment)?;
    
    // Write each payload field
    let mut payload_fields_ptr = payload_fields_segment;
    for field in &proof.execution_payload_header {
        payload_fields_ptr = field.to_memory(vm, payload_fields_ptr)?;
    }
    
    Ok((ptr + 1)?)
}

pub fn assert_epoch_update_result(
    vm: &mut VirtualMachine,
    output_ptr: Relocatable,
    expected_outputs: &ExpectedEpochUpdateCircuitOutputs,
) -> Result<(), HintError> {

    // Check header root (output_ptr + 0, output_ptr + 1)
        let header_root = Uint256::from_memory(vm, output_ptr)?;
        if header_root != expected_outputs.beacon_header_root {
            return Err(HintError::AssertionFailed(format!(
                "Beacon Header Root Mismatch: {:?} != {:?}", 
                header_root, expected_outputs.beacon_header_root
            ).into_boxed_str()));
        }
        
        // Check state root (output_ptr + 2, output_ptr + 3)
        let state_root = Uint256::from_memory(vm, (output_ptr + 2)?)?;
        if state_root != expected_outputs.beacon_state_root {
            return Err(HintError::AssertionFailed(format!(
                "Beacon State Root Mismatch: {:?} != {:?}", 
                state_root, expected_outputs.beacon_state_root
            ).into_boxed_str()));
        }
        
        // Check slot (output_ptr + 4)
        let slot = Felt::from_memory(vm, (output_ptr + 4)?)?;
        if slot != expected_outputs.slot {
            return Err(HintError::AssertionFailed(format!(
                "Slot Mismatch: {:?} != {:?}", 
                slot, expected_outputs.slot
            ).into_boxed_str()));
        }
        
        // Check committee hash (output_ptr + 5, output_ptr + 6)
        let committee_hash = Uint256::from_memory(vm, (output_ptr + 5)?)?;
        if committee_hash != expected_outputs.committee_hash {
            return Err(HintError::AssertionFailed(format!(
                "Committee Hash Mismatch: {:?} != {:?}", 
                committee_hash, expected_outputs.committee_hash
            ).into_boxed_str()));
        }
        
        // Check n_signers (output_ptr + 7)
        let n_signers = Felt::from_memory(vm, (output_ptr + 7)?)?;
        if n_signers != expected_outputs.n_signers {
            return Err(HintError::AssertionFailed(format!(
                "Number of Signers Mismatch: {:?} != {:?}", 
                n_signers, expected_outputs.n_signers
            ).into_boxed_str()));
        }
        
        // Check execution hash (output_ptr + 8, output_ptr + 9)
        let execution_hash = Uint256::from_memory(vm, (output_ptr + 8)?)?;
        if execution_hash != expected_outputs.execution_header_hash {
            return Err(HintError::AssertionFailed(format!(
                "Execution Header Hash Mismatch: {:?} != {:?}", 
                execution_hash, expected_outputs.execution_header_hash
            ).into_boxed_str()));
        }
        
        // Check execution height (output_ptr + 10)
        let execution_height = Felt::from_memory(vm, (output_ptr + 10)?)?;
        if execution_height != expected_outputs.execution_header_height {
            return Err(HintError::AssertionFailed(format!(
                "Execution Header Height Mismatch: {:?} != {:?}", 
                execution_height, expected_outputs.execution_header_height
            ).into_boxed_str()));
        }
    Ok(())
}

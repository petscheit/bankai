use std::collections::HashMap;

use crate::{hint_processor::CustomHintProcessor, types::{Bytes32, Felt, G1CircuitPoint, G2CircuitPoint, UInt384, Uint256, Uint256Bits32}};
use cairo_vm::{hint_processor::builtin_hint_processor::{builtin_hint_processor_definition::HintProcessorData, hint_utils::{get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name}}, types::{exec_scope::ExecutionScopes, relocatable::Relocatable}, vm::{errors::hint_errors::HintError, vm_core::VirtualMachine}, Felt252};
use garaga_zero_hints::types::CairoType;
use num_bigint::BigUint;
use serde::Deserialize;
use beacon_types::{BeaconBlockBody, ExecPayload, ExecutionPayloadHeader, ExecutionPayloadHeaderBellatrix, ExecutionPayloadHeaderCapella, ExecutionPayloadHeaderDeneb, ExecutionPayloadHeaderElectra, MainnetEthSpec};
use serde_json;
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
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_bytes()));
                roots
            },
            ExecutionPayloadHeader::Deneb(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(to_uint256(h.withdrawals_root.as_bytes()));
                roots.push(u64_to_uint256(h.blob_gas_used));
                roots.push(u64_to_uint256(h.excess_blob_gas));
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
            // Write signature point
            let sig_point_ptr = get_relocatable_from_var_name("sig_point", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            epoch_update.circuit_inputs.signature_point.to_memory(vm, sig_point_ptr)?;

            // Write header fields
            self.write_header_fields(vm, hint_data, &epoch_update.circuit_inputs.header)?;
            
            // Write signer data (aggregate pub key and non-signers)
            self.write_signer_data(vm, hint_data, &epoch_update.circuit_inputs)?;

            // Write execution header proof
            self.write_execution_header_proof(vm, hint_data, &epoch_update.circuit_inputs.execution_header_proof)?;
            
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

        let header_root_ptr = get_relocatable_from_var_name("header_root", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let header_root = Uint256::from_memory(vm, header_root_ptr)?;
        if &header_root != &expected_outputs.beacon_header_root {
            return Err(HintError::AssertionFailed(format!("Header Root Mismatch: {:?} != {:?}", header_root, expected_outputs.beacon_header_root).into_boxed_str()));
        }

        let state_root_ptr = get_relocatable_from_var_name("state_root", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let state_root = Uint256::from_memory(vm, state_root_ptr)?;
        if &state_root != &expected_outputs.beacon_state_root {
            return Err(HintError::AssertionFailed(format!("State Root Mismatch: {:?} != {:?}", state_root, expected_outputs.beacon_state_root).into_boxed_str()));
        }

        let committee_hash_ptr = get_relocatable_from_var_name("committee_hash", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let committee_hash = Uint256::from_memory(vm, committee_hash_ptr)?;
        if &committee_hash != &expected_outputs.committee_hash {
            return Err(HintError::AssertionFailed(format!("Committee Hash Mismatch: {:?} != {:?}", committee_hash, expected_outputs.committee_hash).into_boxed_str()));
        }

        let n_signers_ptr = get_relocatable_from_var_name("n_signers", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let n_signers = Felt::from_memory(vm, n_signers_ptr)?;
        if &n_signers != &expected_outputs.n_signers {
            return Err(HintError::AssertionFailed(format!("Number of Signers Mismatch: {:?} != {:?}", n_signers, expected_outputs.n_signers).into_boxed_str()));
        }

        // first word of header is slot.low
        let slot_ptr = get_relocatable_from_var_name("header", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let slot = Felt::from_memory(vm, slot_ptr)?;
        if &slot != &expected_outputs.slot {
            return Err(HintError::AssertionFailed(format!("Slot Mismatch: {:?} != {:?}", slot, expected_outputs.slot).into_boxed_str()));
        }

        let execution_hash_ptr = get_relocatable_from_var_name("execution_hash", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let execution_hash = Uint256::from_memory(vm, execution_hash_ptr)?;
        if &execution_hash != &expected_outputs.execution_header_hash {
            return Err(HintError::AssertionFailed(format!("Execution Header Hash Mismatch: {:?} != {:?}", execution_hash, expected_outputs.execution_header_hash).into_boxed_str()));
        }

        let execution_height_ptr = get_relocatable_from_var_name("execution_height", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let execution_height = Felt::from_memory(vm, execution_height_ptr)?;
        if &execution_height != &expected_outputs.execution_header_height {
            return Err(HintError::AssertionFailed(format!("Execution Header Height Mismatch: {:?} != {:?}", execution_height, expected_outputs.execution_header_height).into_boxed_str()));
        }

        Ok(())
    }

    fn write_header_fields(
        &self,
        vm: &mut VirtualMachine,
        hint_data: &HintProcessorData,
        header: &BeaconHeaderCircuit,
    ) -> Result<(), HintError> {
        let mut header_ptr = get_relocatable_from_var_name("header", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        header_ptr = header.slot.to_memory(vm, header_ptr)?;
        header_ptr = header.proposer_index.to_memory(vm, header_ptr)?;
        header_ptr = header.parent_root.to_memory(vm, header_ptr)?;
        header_ptr = header.state_root.to_memory(vm, header_ptr)?;
        header.body_root.to_memory(vm, header_ptr)?;
        Ok(())
    }

    fn write_signer_data(
        &self,
        vm: &mut VirtualMachine,
        hint_data: &HintProcessorData,
        circuit_inputs: &EpochCircuitInputs,
    ) -> Result<(), HintError> {
        let mut signer_data_ptr = get_relocatable_from_var_name("signer_data", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        
        // Write aggregate public key
        signer_data_ptr = circuit_inputs.aggregate_pub.to_memory(vm, signer_data_ptr)?;
        
        // Create segment for non-signers and store its pointer
        let non_signers_segment = vm.add_memory_segment();
        vm.insert_value(signer_data_ptr, &non_signers_segment)?;

        // Write all non-signers to the segment
        let mut segment_ptr = non_signers_segment;
        for non_signer in &circuit_inputs.non_signers {
            segment_ptr = non_signer.to_memory(vm, segment_ptr)?;
        }
        
        // Store the length of non-signers
        vm.insert_value((signer_data_ptr + 1)?, &Felt252::from(circuit_inputs.non_signers.len()))?;
        
        Ok(())
    }

    fn write_execution_header_proof(
        &self,
        vm: &mut VirtualMachine,
        hint_data: &HintProcessorData,
        proof: &ExecutionHeaderCircuitProof,
    ) -> Result<(), HintError> {
        let mut exec_header_ptr = get_relocatable_from_var_name("execution_header_proof", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        
        // Write root
        exec_header_ptr = proof.root.to_memory(vm, exec_header_ptr)?;
        
        // Create and write path segment
        let path_segment = vm.add_memory_segment();
        vm.insert_value(exec_header_ptr, &path_segment)?;
        exec_header_ptr = (exec_header_ptr + 1)?;
        
        // Write each path element
        let mut path_ptr = path_segment;
        for path_element in &proof.path {
            let element_segment = vm.add_memory_segment();
            vm.insert_value(path_ptr, &element_segment)?;
            path_ptr = (path_ptr + 1)?;
            
            path_element.to_memory(vm, element_segment)?;
        }
        
        // Write leaf and index
        exec_header_ptr = proof.leaf.to_memory(vm, exec_header_ptr)?;
        exec_header_ptr = proof.index.to_memory(vm, exec_header_ptr)?;
        
        // Create and write payload fields segment
        let payload_fields_segment = vm.add_memory_segment();
        vm.insert_value(exec_header_ptr, &payload_fields_segment)?;
        
        // Write each payload field
        let mut payload_fields_ptr = payload_fields_segment;
        for field in &proof.execution_payload_header {
            payload_fields_ptr = field.to_memory(vm, payload_fields_ptr)?;
        }
        
        Ok(())
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

#[derive(Debug, Deserialize)]
pub struct ExpectedEpochUpdateCircuitOutputs {
    pub beacon_header_root: Uint256,
    pub beacon_state_root: Uint256,
    pub committee_hash: Uint256,
    pub n_signers: Felt,
    pub slot: Felt,
    pub execution_header_hash: Uint256,
    pub execution_header_height: Felt,
}
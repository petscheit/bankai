use std::collections::HashMap;

use crate::{hint_processor::CustomHintProcessor, types::{Felt, UInt384, Uint256, Uint256Bits32}};
use cairo_vm::{hint_processor::builtin_hint_processor::{builtin_hint_processor_definition::HintProcessorData, hint_utils::{get_ptr_from_var_name, get_relocatable_from_var_name}}, types::exec_scope::ExecutionScopes, vm::{errors::hint_errors::HintError, vm_core::VirtualMachine}, Felt252};
use garaga_zero_hints::types::CairoType;
use serde::Deserialize;
use serde_json;

#[derive(Deserialize, Debug)]
pub struct CommitteeUpdateCircuit {
    pub circuit_inputs: CircuitInput,
    pub expected_circuit_outputs: CircuitOutput,
}

impl CommitteeUpdateCircuit {
    pub fn from_file(file_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file_content = std::fs::read_to_string(file_path)?;
        let committee_update: CommitteeUpdateCircuit = serde_json::from_str(&file_content)?;
        Ok(committee_update)
    }
    
}

#[derive(Debug, Deserialize)]
pub struct CircuitInput {
    pub beacon_slot: Felt,
    pub next_sync_committee_branch: Vec<Uint256Bits32>,
    pub next_aggregate_sync_committee: UInt384,
    pub committee_keys_root: Uint256Bits32,
}

#[derive(Deserialize, Debug)]
pub struct CircuitOutput {
    pub state_root: Uint256,
    pub slot: Felt,
    pub committee_hash: Uint256,
}

pub const HINT_WRITE_COMMITTEE_UPDATE_INPUTS: &str = r#"write_committee_update_inputs()"#;
pub const HINT_ASSERT_COMMITTEE_UPDATE_RESULT: &str = r#"assert_committee_update_result()"#;

impl CustomHintProcessor {

    pub fn write_committee_update_inputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(committee_update) = &self.committee_input {
    
            let slot_ptr = get_relocatable_from_var_name("slot", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
            committee_update.circuit_inputs.beacon_slot.to_memory(vm, slot_ptr)?;
            
            let aggregate_committee_key_ptr = get_relocatable_from_var_name("aggregate_committee_key", vm, &hint_data.ids_data, &hint_data.ap_tracking).unwrap();
            committee_update.circuit_inputs.next_aggregate_sync_committee.to_memory(vm, aggregate_committee_key_ptr)?;
        
            let committee_keys_root_ptr = get_ptr_from_var_name("committee_keys_root", vm, &hint_data.ids_data, &hint_data.ap_tracking).unwrap();
            committee_update.circuit_inputs.committee_keys_root.to_memory(vm, committee_keys_root_ptr)?;
            
            let path_ptr = get_ptr_from_var_name("path", vm, &hint_data.ids_data, &hint_data.ap_tracking).unwrap();
        
            for (i, branch) in committee_update.circuit_inputs.next_sync_committee_branch.iter().enumerate() {
                let branch_segment = vm.add_memory_segment();
                branch.to_memory(vm, branch_segment)?;
                vm.insert_value((path_ptr + i)?, &branch_segment)?;
            }
        
            let path_len_ptr = get_relocatable_from_var_name("path_len", vm, &hint_data.ids_data, &hint_data.ap_tracking).unwrap();
            let path_len= Felt252::from(committee_update.circuit_inputs.next_sync_committee_branch.len());
            vm.insert_value(path_len_ptr, &path_len)?;
        
            Ok(())
        } else {
            panic!("Committee input not found");
        }
    }

    
    pub fn assert_committee_update_result(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let expected_outputs = &self.committee_input.as_ref().expect("Committee input not found").expected_circuit_outputs;

        let state_root_ptr = get_relocatable_from_var_name("state_root", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let state_root = Uint256::from_memory(vm, state_root_ptr)?;
        let expected_state_root = &expected_outputs.state_root;
        if &state_root != expected_state_root {
            return Err(HintError::AssertionFailed(format!("Invalid state root: {:?} != {:?}", state_root, expected_state_root).into_boxed_str()));
        }

        let slot_ptr = get_relocatable_from_var_name("slot", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let slot = Felt::from_memory(vm, slot_ptr)?;
        let expected_slot = &expected_outputs.slot;
        if &slot != expected_slot {
            return Err(HintError::AssertionFailed(format!("Invalid slot: {:?} != {:?}", slot, expected_slot).into_boxed_str()));
        }

        let committee_hash_ptr = get_relocatable_from_var_name("committee_hash", vm, &hint_data.ids_data, &hint_data.ap_tracking)?;
        let committee_hash = Uint256::from_memory(vm, committee_hash_ptr)?;
        if &committee_hash != &expected_outputs.committee_hash {
            return Err(HintError::AssertionFailed(format!("Invalid committee hash: {:?} != {:?}", committee_hash, expected_outputs.committee_hash).into_boxed_str()));
        }

        Ok(())

    }

}
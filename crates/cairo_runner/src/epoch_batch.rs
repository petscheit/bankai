use cairo_vm::{
    hint_processor::builtin_hint_processor::{
        builtin_hint_processor_definition::HintProcessorData,
        hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name,
        },
    },
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
    Felt252,
};
use garaga_zero::types::CairoType;
use std::collections::HashMap;

use crate::{
    epoch_update::{
        assert_epoch_update_result, write_epoch_update, EpochUpdateCircuit,
        ExpectedEpochUpdateCircuitOutputs,
    },
    hint_processor::CustomHintProcessor,
    types::{Felt, Uint256},
};

pub struct EpochUpdateBatchCircuit {
    pub circuit_inputs: EpochUpdateBatchCircuitInputs,
    pub expected_circuit_outputs: ExpectedEpochUpdateBatchCircuitOutputs,
}

pub struct EpochUpdateBatchCircuitInputs {
    pub committee_hash: Uint256,
    pub epochs: Vec<EpochUpdateCircuit>,
}

pub struct ExpectedEpochUpdateBatchCircuitOutputs {
    pub batch_root: Felt,
    pub latest_batch_output: ExpectedEpochUpdateCircuitOutputs,
}

pub const HINT_WRITE_EPOCH_UPDATE_BATCH_INPUTS: &str = r#"write_epoch_update_batch_inputs()"#;
pub const HINT_ASSERT_BATCHED_EPOCH_OUTPUTS: &str = r#"assert_batched_epoch_outputs()"#;
pub const HINT_ASSERT_EPOCH_BATCH_OUTPUTS: &str = r#"assert_epoch_batch_outputs()"#;

impl CustomHintProcessor {
    pub fn write_epoch_update_batch_inputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        if let Some(epoch_batch) = &self.epoch_batch_input {
            let input = &epoch_batch.circuit_inputs;
            let epoch_batch_ptr = get_relocatable_from_var_name(
                "epoch_batch",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            let mut current_ptr = epoch_batch_ptr;

            current_ptr = input.committee_hash.to_memory(vm, current_ptr)?;

            // Create a segment for the epochs array
            let epochs_segment = vm.add_memory_segment();

            // Store the pointer to the epochs array in the EpochUpdateBatch struct
            vm.insert_value(current_ptr, epochs_segment)?;

            // Now write each epoch to the epochs array
            let mut epoch_ptr = epochs_segment;
            for epoch in &input.epochs {
                epoch_ptr = write_epoch_update(epoch_ptr, &epoch.circuit_inputs, vm)?;
            }

            let batch_len_ptr = get_relocatable_from_var_name(
                "batch_len",
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            )?;
            vm.insert_value(batch_len_ptr, input.epochs.len())?;

            Ok(())
        } else {
            panic!("EpochUpdateBatchCircuit input not found");
        }
    }

    pub fn assert_batched_epoch_outputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let index: usize =
            get_integer_from_var_name("index", vm, &hint_data.ids_data, &hint_data.ap_tracking)?
                .try_into()
                .unwrap();
        let epoch_output_ptr = get_ptr_from_var_name(
            "epoch_output",
            vm,
            &hint_data.ids_data,
            &hint_data.ap_tracking,
        )?;

        if let Some(epoch_batch) = &self.epoch_batch_input {
            let expected_outputs =
                &epoch_batch.circuit_inputs.epochs[index].expected_circuit_outputs;
            assert_epoch_update_result(vm, epoch_output_ptr, expected_outputs)
        } else {
            panic!("EpochUpdateBatchCircuit input not found");
        }
    }

    pub fn assert_epoch_batch_outputs(
        &self,
        vm: &mut VirtualMachine,
        _exec_scopes: &mut ExecutionScopes,
        hint_data: &HintProcessorData,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let output_ptr = (get_ptr_from_var_name(
            "output_ptr",
            vm,
            &hint_data.ids_data,
            &hint_data.ap_tracking,
        )? - 12)?;
        let expected_outputs = &self
            .epoch_batch_input
            .as_ref()
            .unwrap()
            .expected_circuit_outputs;
        let batch_root = Felt::from_memory(vm, output_ptr)?;
        if batch_root != expected_outputs.batch_root {
            return Err(HintError::AssertionFailed(
                format!(
                    "Batch Root Mismatch: {:?} != {:?}",
                    batch_root, expected_outputs.batch_root
                )
                .into_boxed_str(),
            ));
        }

        assert_epoch_update_result(vm, (output_ptr + 1)?, &expected_outputs.latest_batch_output)
    }
}

pub const HINT_SET_NEXT_POWER_OF_2: &str = r#"set_next_power_of_2()"#;

pub fn set_next_power_of_2(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    hint_data: &HintProcessorData,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let batch_len: usize =
        get_integer_from_var_name("batch_len", vm, &hint_data.ids_data, &hint_data.ap_tracking)?
            .try_into()
            .unwrap();
    let mut next_power_of_2: usize = 1;
    let mut power: usize = 0;
    while next_power_of_2 < batch_len {
        next_power_of_2 *= 2;
        power += 1;
    }
    vm.insert_value(
        get_relocatable_from_var_name(
            "next_power_of_2_index",
            vm,
            &hint_data.ids_data,
            &hint_data.ap_tracking,
        )?,
        power,
    )?;
    Ok(())
}

pub const HINT_COMPUTE_EPOCH_FROM_SLOT: &str = r#"compute_epoch_from_slot()"#;
pub fn compute_epoch_from_slot(
    vm: &mut VirtualMachine,
    _exec_scopes: &mut ExecutionScopes,
    hint_data: &HintProcessorData,
    _constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let current_slot: usize = get_integer_from_var_name(
        "current_slot",
        vm,
        &hint_data.ids_data,
        &hint_data.ap_tracking,
    )?
    .try_into()
    .unwrap();

    // Calculate current epoch: slot / 32 (integer division automatically floors)
    let current_epoch = current_slot / 32;
    vm.insert_value(
        get_relocatable_from_var_name(
            "current_epoch",
            vm,
            &hint_data.ids_data,
            &hint_data.ap_tracking,
        )?,
        current_epoch,
    )?;

    Ok(())
}

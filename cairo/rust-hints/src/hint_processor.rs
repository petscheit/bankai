use std::{any::Any, collections::HashMap};

use cairo_vm::{
    Felt252,
    hint_processor::{
        builtin_hint_processor::builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
        hint_processor_definition::{HintExtension, HintProcessorLogic},
    },
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, runners::cairo_runner::ResourceTracker, vm_core::VirtualMachine},
};
use garaga_zero_hints::*;

use crate::committee_update::{CommitteeUpdate};

use crate::committee_update::{HINT_WRITE_CIRCUIT_INPUTS};

pub type HintImpl = fn(&mut VirtualMachine, &mut ExecutionScopes, &HintProcessorData, &HashMap<String, Felt252>) -> Result<(), HintError>;

pub struct CustomHintProcessor {
    hints: HashMap<String, HintImpl>,
    // Add the builtin hint processor
    builtin_hint_proc: BuiltinHintProcessor,
    pub committee_input: Option<CommitteeUpdate>,
}


impl CustomHintProcessor {
    pub fn new(committee_input: Option<CommitteeUpdate>) -> Self {
        Self {
            hints: Self::hints(),
            builtin_hint_proc: BuiltinHintProcessor::new_empty(),
            committee_input,
        }
    }

    fn hints() -> HashMap<String, HintImpl> {
        let mut hints = HashMap::<String, HintImpl>::new();
        hints.insert(modulo_circuit::HINT_RUN_MODULO_CIRCUIT.into(), modulo_circuit::run_modulo_circuit);
        hints.insert(utils::HINT_RETRIEVE_OUTPUT.into(), utils::hint_retrieve_output);
        hints.insert(basic_field_ops::HINT_UINT384_IS_LE.into(), basic_field_ops::hint_uint384_is_le);
        hints.insert(basic_field_ops::HINT_ADD_MOD_CIRCUIT.into(), basic_field_ops::hint_add_mod_circuit);
        hints.insert(basic_field_ops::HINT_NOT_ZERO_MOD_P.into(), basic_field_ops::hint_not_zero_mod_p);
        hints.insert(basic_field_ops::HINT_IS_ZERO_MOD_P.into(), basic_field_ops::hint_is_zero_mod_p);
        hints.insert(
            basic_field_ops::HINT_ASSERT_NEQ_MOD_P.into(),
            basic_field_ops::hint_assert_neq_mod_p,
        );
        hints.insert(
            basic_field_ops::HINT_IS_OPPOSITE_MOD_P.into(),
            basic_field_ops::hint_is_opposite_mod_p,
        );
        hints.insert(hash_to_curve::HINT_MAP_TO_CURVE_G2.into(), hash_to_curve::hint_map_to_curve_g2);
        hints.insert(sha256::HINT_SHA256_FINALIZE.into(), sha256::hint_sha256_finalize);
        hints.insert(debug::PRINT_FELT_HEX.into(), debug::print_felt_hex);
        hints.insert(debug::PRINT_FELT.into(), debug::print_felt);
        hints.insert(debug::PRINT_STRING.into(), debug::print_string);
        hints.insert(debug::PRINT_UINT384.into(), debug::print_uint384);

        
        hints
    }
}

impl HintProcessorLogic for CustomHintProcessor {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        // Delegate to the builtin hint processor
        self.builtin_hint_proc.execute_hint(vm, exec_scopes, hint_data, constants)
    }

    fn execute_hint_extensive(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<HintExtension, HintError> {
        if let Some(hpd) = hint_data.downcast_ref::<HintProcessorData>() {
            let hint_code = hpd.code.as_str();

            let res = match hint_code {
                HINT_WRITE_CIRCUIT_INPUTS => self.write_circuit_inputs(vm, exec_scopes, hpd, constants),
                _ => Err(HintError::UnknownHint(hint_code.to_string().into_boxed_str())),
            };

            if !matches!(res, Err(HintError::UnknownHint(_))) {
                return res.map(|_| HintExtension::default());
            }
            println!("Hint code: {}", hint_code);

            // First try our custom hints
            if let Some(hint_impl) = self.hints.get(hint_code) {
                return hint_impl(vm, exec_scopes, hpd, constants).map(|_| HintExtension::default());
            }

            // If not found, try the builtin hint processor
            return self
                .builtin_hint_proc
                .execute_hint(vm, exec_scopes, hint_data, constants)
                .map(|_| HintExtension::default());
        }

        // For other hint types (like Cairo 1 hints), you might need additional handling here
        Err(HintError::WrongHintData)
    }
}

impl ResourceTracker for CustomHintProcessor {}

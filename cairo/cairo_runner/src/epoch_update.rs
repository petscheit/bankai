use std::collections::HashMap;

use crate::{hint_processor::CustomHintProcessor, types::{Felt, UInt384, Uint256, Uint256Bits32}};
use cairo_vm::{hint_processor::builtin_hint_processor::{builtin_hint_processor_definition::HintProcessorData, hint_utils::{get_ptr_from_var_name, get_relocatable_from_var_name}}, types::exec_scope::ExecutionScopes, vm::{errors::hint_errors::HintError, vm_core::VirtualMachine}, Felt252};
use garaga_zero_hints::types::CairoType;
use num_bigint::BigUint;
use serde::Deserialize;
use beacon_types::{BeaconBlockBody, ExecPayload, ExecutionPayloadHeader, ExecutionPayloadHeaderBellatrix, ExecutionPayloadHeaderCapella, ExecutionPayloadHeaderDeneb, ExecutionPayloadHeaderElectra, MainnetEthSpec};
use serde_json;
use beacon_types::TreeHash;

pub struct EpochUpdate {
    pub circuit_inputs: EpochCircuitInputs,
    // pub expected_circuit_outputs: ExpectedEpochUpdateOutputs,
}

#[derive(Debug, Deserialize)]
pub struct EpochCircuitInputs {
    pub header: BeaconHeaderCircuit,
    pub signature_point: G2CircuitPoint,
    pub aggregate_pub: G1CircuitPoint,
    pub non_signers: Vec<G1CircuitPoint>,
    // pub execution_header_proof: ExecutionHeaderProof,
}


#[derive(Debug, Deserialize)]
pub struct BeaconHeaderCircuit {
    pub slot: Felt,
    pub proposer_index: Felt,
    pub parent_root: Uint256,
    pub state_root: Uint256,
    pub body_root: Uint256,
}

#[derive(Debug, Deserialize)]
pub struct G1CircuitPoint{
    x: UInt384,
    y: UInt384,
}

#[derive(Debug, Deserialize)]
pub struct G2CircuitPoint{
    x0: UInt384,
    x1: UInt384,
    y0: UInt384,
    y1: UInt384,
}

pub struct ExecutionHeaderCircuitProof {
    pub root: Uint256,
    pub path: Vec<Uint256Bits32>,
    pub leaf: Uint256,
    pub index: Felt,
    pub execution_payload_header: ExecutionPayloadHeaderCircuit,
}

pub struct ExecutionPayloadHeaderCircuit(pub ExecutionPayloadHeader<MainnetEthSpec>);

impl ExecutionPayloadHeaderCircuit {
    pub fn to_field_roots(&self) -> Vec<Uint256> {
        macro_rules! extract_common_fields {
            ($h:expr) => {
                vec![
                    Uint256(BigUint::from_bytes_be(&$h.parent_hash.0.as_bytes())),
                    Uint256(BigUint::from_bytes_be(&$h.fee_recipient.0.to_vec())),
                    Uint256(BigUint::from_bytes_be(&$h.state_root.0.to_vec())),
                    Uint256(BigUint::from_bytes_be(&$h.receipts_root.0.to_vec())),
                    Uint256(BigUint::from_bytes_be(&$h.logs_bloom.tree_hash_root().as_bytes())),
                    Uint256(BigUint::from_bytes_be(&$h.prev_randao.0.to_vec())),
                    Uint256(BigUint::from($h.block_number)),
                    Uint256(BigUint::from($h.gas_limit)),
                    Uint256(BigUint::from($h.gas_used)),
                    Uint256(BigUint::from($h.timestamp)),
                    Uint256(BigUint::from_bytes_be(&$h.extra_data.tree_hash_root().as_bytes())),
                    Uint256(BigUint::from_bytes_be(&$h.base_fee_per_gas.tree_hash_root().as_bytes())),
                    Uint256(BigUint::from_bytes_be(&$h.block_hash.0.as_bytes())),
                    Uint256(BigUint::from_bytes_be(&$h.transactions_root.as_bytes())),
                ]
            };
        }

        let roots = match &self.0 {
            ExecutionPayloadHeader::Bellatrix(h) => extract_common_fields!(h),
            ExecutionPayloadHeader::Capella(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(Uint256(BigUint::from_bytes_be(&h.withdrawals_root.as_bytes())));
                roots
            },
            ExecutionPayloadHeader::Deneb(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(Uint256(BigUint::from_bytes_be(&h.withdrawals_root.as_bytes())));
                roots.push(Uint256(BigUint::from(h.blob_gas_used)));
                roots.push(Uint256(BigUint::from(h.excess_blob_gas)));
                roots
            },
            ExecutionPayloadHeader::Electra(h) => {
                let mut roots = extract_common_fields!(h);
                roots.push(Uint256(BigUint::from_bytes_be(&h.withdrawals_root.as_bytes())));
                roots.push(Uint256(BigUint::from(h.blob_gas_used)));
                roots.push(Uint256(BigUint::from(h.excess_blob_gas)));
                roots.push(Uint256(BigUint::from_bytes_be(&h.deposit_requests_root.as_bytes())));
                roots.push(Uint256(BigUint::from_bytes_be(&h.withdrawal_requests_root.as_bytes())));
                roots
            },
        };

        roots
    }
}
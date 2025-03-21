use super::proofs::ProofError;
use starknet::core::types::Felt;

/// A trait for the types that can be submitted on-chain
pub trait Submittable<T> {
    fn get_contract_selector(&self) -> Felt;
    fn to_calldata(&self) -> Vec<Felt>;
    fn from_inputs(circuit_inputs: &T) -> Self;
}

pub enum ProofType {
    Epoch,
    EpochBatch,
    SyncCommittee,
}

pub trait Exportable {
    fn export(&self) -> Result<String, ProofError>;
}

use serde::Serialize;
use starknet::core::types::Felt;

use crate::Error;

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

pub trait Provable: Serialize {
    fn id(&self) -> String;
    fn export(&self) -> Result<String, Error>;
    // fn from_json<T>(slot: u64) -> Result<T, Error>
    // where
    //     T: serde::de::DeserializeOwned;
    fn proof_type(&self) -> ProofType;
    fn pie_path(&self) -> String;
    fn inputs_path(&self) -> String;
}

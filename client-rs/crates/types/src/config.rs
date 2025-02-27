use crate::constants::{
    MAX_CONCURRENT_PIE_GENERATIONS, MAX_CONCURRENT_RPC_DATA_FETCH_JOBS, STARKNET_SEPOLIA,
};
use starknet::core::types::Felt;
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone, Debug)]
pub struct BankaiConfig {
    pub contract_class_hash: Felt,
    pub contract_address: Felt,
    pub committee_update_program_hash: Felt,
    pub epoch_update_program_hash: Felt,
    pub epoch_batch_program_hash: Felt,
    pub contract_path: String,
    pub epoch_circuit_path: String,
    pub epoch_batch_circuit_path: String,
    pub committee_circuit_path: String,
    pub atlantic_endpoint: String,
    pub transactor_endpoint: String,
    pub pie_generation_semaphore: Arc<Semaphore>,
    pub epoch_data_fetching_semaphore: Arc<Semaphore>,
    pub proof_settlement_chain_id: Felt,
}

impl Default for BankaiConfig {
    fn default() -> Self {
        Self {
            contract_class_hash: Felt::from_hex(
                "0x00034b6d1cd9858aeabcee33ef5ec5cd04be155d79ca2bbf9036700cb6c7c287",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0x1b7b70023bc2429d4453ce75d75f3e8b01b0730ca83068a82b4d17aa88a25e3",
            )
            .unwrap(),
            committee_update_program_hash: Felt::from_hex(
                "0x229e5ad2e3b8c6dd4d0319cdd957bbd7bdf2ea685e172b049c3e5f55b0352c1",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x5daec246cf8296195084c05ca21ee0f77452c39e635232565557a9f3ce9f596",
            )
            .unwrap(),
            epoch_batch_program_hash: Felt::from_hex(
                "0x5f4dad2d8549e91c25694875eb02fc2910eeead0e1a13d3061464a3eaa4bd8d",
            )
            .unwrap(),
            contract_path: "../contract/target/release/bankai_BankaiContract.contract_class.json"
                .to_string(),
            epoch_circuit_path: "../cairo/build/epoch_update.json".to_string(),
            epoch_batch_circuit_path: "../cairo/build/epoch_batch.json".to_string(),
            committee_circuit_path: "../cairo/build/committee_update.json".to_string(),
            atlantic_endpoint: "https://atlantic.api.herodotus.cloud".to_string(),
            transactor_endpoint: "https://staging.api.herodotus.cloud".to_string(),
            // Set how many concurrent pie generation (trace generation) tasks are allowed
            pie_generation_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_PIE_GENERATIONS)), // 3 at once
            epoch_data_fetching_semaphore: Arc::new(Semaphore::new(
                MAX_CONCURRENT_RPC_DATA_FETCH_JOBS,
            )), // 2 at once
            proof_settlement_chain_id: Felt::from_hex(STARKNET_SEPOLIA).unwrap(),
        }
    }
}

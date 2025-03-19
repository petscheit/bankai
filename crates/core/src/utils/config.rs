use crate::utils::constants::{
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
    pub cairo_verifier_path: String,
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
                "0x5ed209e0e60206c61e81b708d202e5c34a6184f3eb749ea9c5a2139d8997ae0",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x03b515930ea20dcf5115ca00c6f7084220a45395c8f48e170e6ff4bcc27c5d4d",
            )
            .unwrap(),
            epoch_batch_program_hash: Felt::from_hex(
                "0x82c00cdcb19e0248ab95bff1045d1296c309e9a63bf9ff76af03175369d71d",
            )
            .unwrap(),
            contract_path:
                "../../contract/target/release/bankai_BankaiContract.contract_class.json"
                    .to_string(),
            epoch_circuit_path: "../../cairo/build/epoch_update.json".to_string(),
            epoch_batch_circuit_path: "../../cairo/build/epoch_batch.json".to_string(),
            committee_circuit_path: "../../cairo/build/committee_update.json".to_string(),
            cairo_verifier_path: "../../cairo/verifier/cairo_verifier.json".to_string(),
            atlantic_endpoint: "https://staging.atlantic.api.herodotus.cloud".to_string(),
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

impl BankaiConfig {
    pub fn docker_config() -> Self {
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
                "0x5ed209e0e60206c61e81b708d202e5c34a6184f3eb749ea9c5a2139d8997ae0",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x03b515930ea20dcf5115ca00c6f7084220a45395c8f48e170e6ff4bcc27c5d4d",
            )
            .unwrap(),
            epoch_batch_program_hash: Felt::from_hex(
                "0x82c00cdcb19e0248ab95bff1045d1296c309e9a63bf9ff76af03175369d71d",
            )
            .unwrap(),
            contract_path:
                "../../contract/target/release/bankai_BankaiContract.contract_class.json"
                    .to_string(),
            epoch_circuit_path: "/app/cairo/build/epoch_update.json".to_string(),
            epoch_batch_circuit_path: "/app/cairo/build/epoch_batch.json".to_string(),
            committee_circuit_path: "/app/cairo/build/committee_update.json".to_string(),
            cairo_verifier_path: "/app/cairo/verifier/cairo_verifier.json".to_string(),
            atlantic_endpoint: "https://staging.atlantic.api.herodotus.cloud".to_string(),
            transactor_endpoint: "https://staging.api.herodotus.cloud".to_string(),
            pie_generation_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_PIE_GENERATIONS)), // 3 at once
            epoch_data_fetching_semaphore: Arc::new(Semaphore::new(
                MAX_CONCURRENT_RPC_DATA_FETCH_JOBS,
            )), // 2 at once
            proof_settlement_chain_id: Felt::from_hex(STARKNET_SEPOLIA).unwrap(),
        }
    }

    pub fn test_config() -> Self {
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
                "0x5ed209e0e60206c61e81b708d202e5c34a6184f3eb749ea9c5a2139d8997ae0",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x03b515930ea20dcf5115ca00c6f7084220a45395c8f48e170e6ff4bcc27c5d4d",
            )
            .unwrap(),
            epoch_batch_program_hash: Felt::from_hex(
                "0x82c00cdcb19e0248ab95bff1045d1296c309e9a63bf9ff76af03175369d71d",
            )
            .unwrap(),
            contract_path: "./test_data/bankai_contract.json".to_string(),
            epoch_circuit_path: "../cairo/build/epoch_update.json".to_string(),
            epoch_batch_circuit_path: "../cairo/build/epoch_batch.json".to_string(),
            committee_circuit_path: "../cairo/build/committee_update.json".to_string(),
            cairo_verifier_path: "./test_data/cairo_verifier.json".to_string(),
            atlantic_endpoint: "https://staging.atlantic.api.herodotus.cloud".to_string(),
            transactor_endpoint: "https://staging.api.herodotus.cloud".to_string(),
            pie_generation_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_PIE_GENERATIONS)),
            epoch_data_fetching_semaphore: Arc::new(Semaphore::new(
                MAX_CONCURRENT_RPC_DATA_FETCH_JOBS,
            )),
            proof_settlement_chain_id: Felt::from_hex(STARKNET_SEPOLIA).unwrap(),
        }
    }
}

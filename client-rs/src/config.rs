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
    pub pie_generation_semaphore: Arc<Semaphore>,
    pub epoch_data_fetching_semaphore: Arc<Semaphore>,
}

impl Default for BankaiConfig {
    fn default() -> Self {
        Self {
            contract_class_hash: Felt::from_hex(
                "0x02b5b08b233132464c437cf15509338e65ae7acc20419a37a9449a1d8e927f46",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0x440b622a97fab3f31a35e7e710a8a508f6693d61d74171b5c2304f5e37ccde8",
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
                "0x19bc492f1036c889939a5174e8f77ffbe89676c8d5f1adef0a825d2a6cc2a2f",
            )
            .unwrap(),
            contract_path: "../contract/target/release/bankai_BankaiContract.contract_class.json"
                .to_string(),
            epoch_circuit_path: "../cairo/build/epoch_update.json".to_string(),
            epoch_batch_circuit_path: "../cairo/build/epoch_batch.json".to_string(),
            committee_circuit_path: "../cairo/build/committee_update.json".to_string(),
            atlantic_endpoint: "https://atlantic.api.herodotus.cloud".to_string(),
            // Set how many concurrent pie generation (trace generation) tasks are allowed
            pie_generation_semaphore: Arc::new(Semaphore::new(1)), // 3 at once
            epoch_data_fetching_semaphore: Arc::new(Semaphore::new(2)), // 2 at once
        }
    }
}

use starknet::core::types::Felt;

#[derive(Clone)]
pub struct BankaiConfig {
    pub contract_class_hash: Felt,
    pub contract_address: Felt,
    pub committee_update_program_hash: Felt,
    pub epoch_update_program_hash: Felt,
    pub contract_path: String,
    pub epoch_circuit_path: String,
    pub committee_circuit_path: String,
    pub atlantic_endpoint: String,
}

impl Default for BankaiConfig {
    fn default() -> Self {
        Self {
            contract_class_hash: Felt::from_hex(
                "0x052c05f5027ad8f963168ebdf9d1518c938648681e43edd00807c28a71ea0b6a",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0x3c36fad01f7a9a8e893e7983a80bffb9ff81079b30f56703cff75a2347d619f",
            )
            .unwrap(),
            committee_update_program_hash: Felt::from_hex(
                "0x229e5ad2e3b8c6dd4d0319cdd957bbd7bdf2ea685e172b049c3e5f55b0352c1",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x61c9a8dc4629396452bffa605c59c947a4a344d85c6496f591787f2b6c422db",
            )
            .unwrap(),
            contract_path: "../contract/target/release/bankai_BankaiContract.contract_class.json"
                .to_string(),
            epoch_circuit_path: "../cairo/build/epoch_update.json".to_string(),
            committee_circuit_path: "../cairo/build/committee_update.json".to_string(),
            atlantic_endpoint: "https://atlantic.api.herodotus.cloud".to_string(),
        }
    }
}

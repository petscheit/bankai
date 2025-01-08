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
                "0x7b2245b6c3f824ec63b28cdcc0405890811f605eafbfb136cb1bea4cffdab9d",
            )
            .unwrap(),
            contract_address: Felt::from_hex(
                "0x5b16f63a4165bad1a247df2d27d8068aed713da76833811580f92ca357bcf0c",
            )
            .unwrap(),
            committee_update_program_hash: Felt::from_hex(
                "0x229e5ad2e3b8c6dd4d0319cdd957bbd7bdf2ea685e172b049c3e5f55b0352c1",
            )
            .unwrap(),
            epoch_update_program_hash: Felt::from_hex(
                "0x79bbdf50c8e723c29a8f560b2be88e39749c5f7f5f6cc691f3ed1bbfe137a80",
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

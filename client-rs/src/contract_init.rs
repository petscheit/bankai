use crate::{types::ContractInitializationData, utils::rpc::BeaconRpcClient, BankaiConfig, Error};

impl ContractInitializationData {
    pub async fn generate_contract_initialization_data(client: &BeaconRpcClient, slot: u64, config: &BankaiConfig) -> Result<Self, Error> {
        let committee = client.get_sync_committee_validator_pubs(slot).await?;
        Ok(Self {
            committee_id: (slot / 0x2000) as u64,
            committee_hash: committee.get_committee_hash(),
            committee_update_program_hash: config.committee_update_program_hash,
            epoch_update_program_hash: config.epoch_update_program_hash,
        })
    }
}

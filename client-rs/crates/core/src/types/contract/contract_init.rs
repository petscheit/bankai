use crate::clients::beacon_chain::{BeaconError, BeaconRpcClient};
use crate::utils::config::BankaiConfig;
use alloy_primitives::FixedBytes;
use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;

#[derive(Debug, Serialize, Deserialize)]
pub struct ContractInitializationData {
    pub(crate) committee_id: u64,
    pub(crate) committee_hash: FixedBytes<32>,
    pub(crate) committee_update_program_hash: Felt,
    pub(crate) epoch_update_program_hash: Felt,
    pub(crate) epoch_batch_program_hash: Felt,
}

impl ContractInitializationData {
    pub async fn new(
        client: &BeaconRpcClient,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<Self, ContractInitializationError> {
        let committee = client.get_sync_committee_validator_pubs(slot).await?;
        Ok(Self {
            committee_id: (slot / 0x2000), // since this is the current committee, we dont increment the committee id
            committee_hash: committee.get_committee_hash(),
            committee_update_program_hash: config.committee_update_program_hash,
            epoch_update_program_hash: config.epoch_update_program_hash,
            epoch_batch_program_hash: config.epoch_batch_program_hash,
        })
    }

    pub fn to_calldata(&self) -> Vec<Felt> {
        let (committee_high, committee_low) = self.committee_hash.as_slice().split_at(16);
        vec![
            Felt::from(self.committee_id),
            Felt::from_bytes_be_slice(committee_low),
            Felt::from_bytes_be_slice(committee_high),
            self.committee_update_program_hash,
            self.epoch_update_program_hash,
            self.epoch_batch_program_hash,
        ]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ContractInitializationError {
    #[error("Beacon error: {0}")]
    Beacon(#[from] BeaconError),
}

pub mod cairo_runner;
pub mod clients;
pub mod db;
pub mod types;
pub mod utils;

use crate::utils::constants;
use crate::{
    clients::atlantic::AtlanticClient, clients::beacon_chain::BeaconRpcClient,
    clients::starknet::StarknetClient, clients::transactor::TransactorClient,
    types::contract::contract_init::ContractInitializationData, types::contract::ContractError,
    types::error::BankaiCoreError, types::proofs::epoch_update::EpochUpdate,
    types::proofs::sync_committee::SyncCommitteeUpdate, types::proofs::ProofError,
    utils::config::BankaiConfig,
};
use clients::beacon_chain::BeaconError;
use clients::ClientError;
use dotenv::from_filename;
use std::env;
use tracing::info;

#[derive(Debug)]
pub struct BankaiClient {
    pub client: BeaconRpcClient,
    pub starknet_client: StarknetClient,
    pub config: BankaiConfig,
    pub atlantic_client: AtlanticClient,
    pub transactor_client: TransactorClient,
}

impl BankaiClient {
    pub async fn new(is_docker: bool) -> Self {
        let config = if is_docker {
            BankaiConfig::docker_config()
        } else {
            from_filename(".env.sepolia").ok();
            BankaiConfig::default()
        };

        Self {
            client: BeaconRpcClient::new(env::var("BEACON_RPC_URL").unwrap(), config.clone()),
            starknet_client: StarknetClient::new(
                env::var("STARKNET_RPC_URL").unwrap().as_str(),
                env::var("STARKNET_ADDRESS").unwrap().as_str(),
                env::var("STARKNET_PRIVATE_KEY").unwrap().as_str(),
            )
            .await
            .unwrap(),
            atlantic_client: AtlanticClient::new(
                config.atlantic_endpoint.clone(),
                env::var("ATLANTIC_API_KEY").unwrap(),
            ),
            transactor_client: TransactorClient::new(
                config.transactor_endpoint.clone(),
                env::var("TRANSACTOR_API_KEY").unwrap(),
            ),
            config,
        }
    }

    pub async fn get_sync_committee_update(
        &self,
        mut slot: u64,
    ) -> Result<SyncCommitteeUpdate, BankaiCoreError> {
        let mut attempts = 0;

        // Before we start generating the proof, we ensure the slot was not missed
        let _header = loop {
            match self.client.get_header(slot).await {
                Ok(header) => break header,
                Err(BeaconError::EmptySlot(_)) => {
                    attempts += 1;
                    if attempts >= constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS {
                        return Err(BankaiCoreError::Client(ClientError::Beacon(
                            BeaconError::EmptySlot(slot),
                        )));
                    }
                    slot += 1;
                    info!(
                        "Empty slot detected! Attempt {}/{}. Fetching slot: {}",
                        attempts,
                        constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS,
                        slot
                    );
                }
                Err(e) => return Err(BankaiCoreError::Client(ClientError::Beacon(e))),
            }
        };

        let proof = SyncCommitteeUpdate::new(&self.client, slot)
            .await
            .map_err(|e| BankaiCoreError::Proof(ProofError::SyncCommittee(e)))?;

        Ok(proof)
    }

    pub async fn get_epoch_proof(&self, slot: u64) -> Result<EpochUpdate, BankaiCoreError> {
        let epoch_proof = EpochUpdate::new(&self.client, slot)
            .await
            .map_err(|e| BankaiCoreError::Proof(ProofError::EpochUpdate(e)))?;
        Ok(epoch_proof)
    }

    pub async fn get_contract_initialization_data(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<ContractInitializationData, BankaiCoreError> {
        let contract_init = ContractInitializationData::new(&self.client, slot, config)
            .await
            .map_err(|e| BankaiCoreError::Contract(ContractError::Initialization(e)))?;
        Ok(contract_init)
    }
}

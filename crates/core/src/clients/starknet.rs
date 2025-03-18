//! StarkNet Client Implementation
//!
//! This module provides a high-level client for interacting with the StarkNet blockchain.
//! It handles contract deployment, transaction submission, and various query operations
//! through a StarkNet RPC endpoint.

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

use starknet::{
    accounts::{Account, ConnectedAccount, ExecutionEncoding, SingleOwnerAccount},
    contract::ContractFactory,
    core::{
        chain_id,
        types::{
            contract::SierraClass, BlockId, BlockTag, Call, Felt, FromStrError, FunctionCall,
            TransactionExecutionStatus, TransactionStatus,
        },
    },
    macros::{felt, selector},
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider, ProviderError, Url,
    },
    signers::{LocalWallet, SigningKey},
};

use crate::{
    types::contract::contract_init::ContractInitializationData,
    types::proofs::epoch_update::EpochProof, types::traits::Submittable,
    utils::config::BankaiConfig, utils::constants,
};
use thiserror::Error;

/// Main client struct for interacting with StarkNet
///
/// Provides high-level access to StarkNet operations through a connected account.
/// The client handles transaction signing, submission and status tracking.
#[derive(Debug)]
pub struct StarknetClient {
    /// Connected account with signing capabilities
    account: Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
    // provider: Arc<JsonRpcClient<HttpTransport>>,
}

/// Possible errors that can occur during StarkNet operations
#[derive(Debug, Error)]
pub enum StarknetError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("Account error: {0}")]
    AccountError(String),
    #[error("Transaction error: {0}")]
    TransactionError(String),
    #[error("Timeout error")]
    TimeoutError,
    #[error("Url parse error")]
    UrlParseError,
    #[error("Felt parse error")]
    FeltParseError(#[from] FromStrError),
    #[error("Io error")]
    IoError(#[from] std::io::Error),
    #[error("Serde json error")]
    SerdeJson(#[from] serde_json::Error),
}

impl StarknetClient {
    /// Creates a new StarkNet client instance
    ///
    /// # Arguments
    /// * `rpc_url` - URL of the StarkNet RPC endpoint
    /// * `address` - Account address in hex format
    /// * `priv_key` - Private key in hex format
    ///
    /// # Returns
    /// * `Result<Self, StarknetError>` - New client instance or error
    pub async fn new(rpc_url: &str, address: &str, priv_key: &str) -> Result<Self, StarknetError> {
        let url = Url::parse(rpc_url).map_err(|_| StarknetError::UrlParseError)?;
        let provider = JsonRpcClient::new(HttpTransport::new(url));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(Felt::from_hex(priv_key)?));
        let address = Felt::from_hex(address)?;
        let mut account = SingleOwnerAccount::new(
            provider,
            signer,
            address,
            chain_id::SEPOLIA,
            ExecutionEncoding::New,
        );

        account.set_block_id(BlockId::Tag(BlockTag::Pending));

        Ok(Self {
            account: Arc::new(account),
        })
    }

    /// Deploys a new contract to StarkNet
    ///
    /// # Arguments
    /// * `init_data` - Contract initialization data
    /// * `config` - Configuration containing contract details
    ///
    /// # Returns
    /// * `Result<Felt, StarknetError>` - Contract address or error
    pub async fn deploy_contract(
        &self,
        init_data: ContractInitializationData,
        config: &BankaiConfig,
    ) -> Result<Felt, StarknetError> {
        let contract_artifact: SierraClass =
            serde_json::from_reader(std::fs::File::open(config.contract_path.clone())?)?;
        let class_hash = contract_artifact.class_hash().unwrap();
        assert!(
            class_hash == config.contract_class_hash,
            "Contract class hash mismatch! Please update config: {:?}",
            class_hash
        );

        let mut params = init_data.to_calldata();
        params.push(self.account.address());

        let contract_factory = ContractFactory::new(class_hash, self.account.clone());
        let deploy_tx = contract_factory.deploy_v1(params, felt!("1337"), false);

        let contract_address = deploy_tx.deployed_address();

        assert!(
            contract_address == config.contract_address,
            "Contract address mismatch! Please update config: {:?}",
            contract_address
        );

        match deploy_tx.send().await {
            Ok(_result) => {
                info!("Deployment transaction sent successfully");
                Ok(contract_address)
            }
            Err(e) => Err(StarknetError::AccountError(format!(
                "Deployment failed: {:#?}",
                e
            ))),
        }
    }

    /// Submits an update transaction to a deployed contract
    ///
    /// # Arguments
    /// * `update` - Update data implementing the Submittable trait
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<Felt, StarknetError>` - Transaction hash or error
    pub async fn submit_update<T>(
        &self,
        update: impl Submittable<T>,
        config: &BankaiConfig,
    ) -> Result<Felt, StarknetError> {
        let selector = update.get_contract_selector();
        let calldata = update.to_calldata();

        let call = Call {
            to: config.contract_address,
            selector,
            calldata,
        };

        let send_result = self.account.execute_v1(vec![call]).send().await;

        match send_result {
            Ok(tx_response) => {
                let tx_hash = tx_response.transaction_hash;
                info!("Transaction sent successfully! Hash: {:#x}", tx_hash);
                Ok(tx_hash)
            }
            Err(e) => {
                error!("Transaction execution error: {:#?}", e);
                Err(StarknetError::TransactionError(format!(
                    "TransactionExecutionError: {:#?}",
                    e
                )))
            }
        }
    }

    /// Retrieves the committee hash for a given slot
    ///
    /// # Arguments
    /// * `slot` - Slot number to query
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<Vec<Felt>, StarknetError>` - Committee hash or error
    pub async fn get_committee_hash(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<Vec<Felt>, StarknetError> {
        let committee_id = slot / 0x2000_u64;
        let committee_hash = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: config.contract_address,
                    entry_point_selector: selector!("get_committee_hash"),
                    calldata: vec![committee_id.into()],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;
        Ok(committee_hash)
    }

    /// Retrieves epoch proof for a given slot
    ///
    /// # Arguments
    /// * `slot` - Slot number to query
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<EpochProof, StarknetError>` - Epoch proof or error
    pub async fn get_epoch_proof(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<EpochProof, StarknetError> {
        let epoch_proof = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: config.contract_address,
                    entry_point_selector: selector!("get_epoch_proof"),
                    calldata: vec![slot.into()],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;
        Ok(EpochProof::from_contract_return_value(epoch_proof).unwrap())
    }

    /// Gets the latest epoch slot number from the contract
    ///
    /// # Arguments
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<Felt, StarknetError>` - Latest epoch slot or error
    pub async fn get_latest_epoch_slot(
        &self,
        config: &BankaiConfig,
    ) -> Result<Felt, StarknetError> {
        let latest_epoch = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: config.contract_address,
                    entry_point_selector: selector!("get_latest_epoch_slot"),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;
        Ok(*latest_epoch.first().unwrap())
    }

    /// Computes the slot range for the current term
    ///
    /// # Arguments
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<(u64, u64), StarknetError>` - Tuple of (next_epoch_slot, terms_last_epoch_slot)
    pub async fn get_batching_range(
        &self,
        config: &BankaiConfig,
    ) -> Result<(u64, u64), StarknetError> {
        let latest_epoch_slot = self.get_latest_epoch_slot(config).await?;
        let next_epoch_slot = (u64::try_from(latest_epoch_slot).unwrap() / 32) * 32 + 32;
        let term = next_epoch_slot / 0x2000;
        let terms_last_epoch_slot = (term + 1) * 0x2000 - 32;
        Ok((next_epoch_slot, terms_last_epoch_slot))
    }

    /// Gets the latest committee ID from the contract
    ///
    /// # Arguments
    /// * `config` - Configuration containing contract address
    ///
    /// # Returns
    /// * `Result<Felt, StarknetError>` - Latest committee ID or error
    pub async fn get_latest_committee_id(
        &self,
        config: &BankaiConfig,
    ) -> Result<Felt, StarknetError> {
        let latest_committee_id = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: config.contract_address,
                    entry_point_selector: selector!("get_latest_committee_id"),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await?;

        Ok(*latest_committee_id.first().unwrap())
    }

    /// Waits for transaction confirmation with retries
    ///
    /// # Arguments
    /// * `tx_hash` - Transaction hash to monitor
    ///
    /// # Returns
    /// * `Result<(), StarknetError>` - Success or error after max retries
    pub async fn wait_for_confirmation(&self, tx_hash: Felt) -> Result<(), StarknetError> {
        let max_retries = constants::STARKNET_TX_CONFIRMATION_MAX_RETRIES;
        let delay = Duration::from_secs(constants::STARKNET_TX_CONFIRMATION_CHECK_DELAY);

        for attempt in 0..max_retries {
            match self.get_transaction_status(tx_hash).await {
                Ok(status) => {
                    info!("Starknet transaction status: {:?}", status);
                    match status {
                        TransactionStatus::AcceptedOnL1(TransactionExecutionStatus::Succeeded)
                        | TransactionStatus::AcceptedOnL2(TransactionExecutionStatus::Succeeded) => {
                            info!("Starknet transaction confirmed: {:?}", tx_hash);
                            return Ok(());
                        }
                        TransactionStatus::Rejected => {
                            return Err(StarknetError::TransactionError(
                                "Transaction rejected".to_string(),
                            ));
                        }
                        _ => {
                            info!(
                                "Transaction is still pending (attempt {} of {}), sleeping...",
                                attempt + 1,
                                max_retries
                            );
                            sleep(delay).await;
                        }
                    }
                }
                Err(err) => {
                    // If the transaction hash is not even found yet, or other unknown error

                    error!(
                        "Error fetching transaction status for tx_hash={:?}: {:?}",
                        tx_hash, err
                    );

                    sleep(delay).await;
                }
            }
        }

        Err(StarknetError::TimeoutError)
    }

    /// Gets the current status of a transaction
    ///
    /// # Arguments
    /// * `tx_hash` - Transaction hash to query
    ///
    /// # Returns
    /// * `Result<TransactionStatus, StarknetError>` - Transaction status or error
    pub async fn get_transaction_status(
        &self,
        tx_hash: Felt,
    ) -> Result<TransactionStatus, StarknetError> {
        let provider = self.account.provider();
        let tx_status = provider.get_transaction_status(tx_hash).await?;

        Ok(tx_status)
    }
}

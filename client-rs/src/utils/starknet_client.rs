use alloy_primitives::FixedBytes;
use serde::{Deserialize, Serialize};
use starknet::accounts::{Account, ConnectedAccount};
use starknet::core::types::{Call, FunctionCall};
use starknet::macros::selector;
use starknet::providers::{Provider, ProviderError};
use tracing::{debug, error, info, trace};

use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    contract::ContractFactory,
    core::{
        chain_id,
        types::{
            contract::SierraClass, BlockId, BlockTag, Felt, TransactionExecutionStatus,
            TransactionStatus,
        },
    },
    macros::felt,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Url,
    },
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::contract_init::ContractInitializationData;
use crate::traits::Submittable;
use crate::BankaiConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochProof {
    pub header_root: FixedBytes<32>,
    pub state_root: FixedBytes<32>,
    pub n_signers: u64,
    pub execution_hash: FixedBytes<32>,
    pub execution_height: u64,
}

impl EpochProof {
    pub fn from_contract_return_value(calldata: Vec<Felt>) -> Result<Self, String> {
        if calldata.len() != 8 {
            return Err("Invalid return value length. Expected 8 elements.".to_string());
        }

        let header_root = combine_to_fixed_bytes(calldata[0], calldata[1])?;
        let state_root = combine_to_fixed_bytes(calldata[2], calldata[3])?;
        let n_signers = calldata[4].try_into().unwrap();
        let execution_hash = combine_to_fixed_bytes(calldata[5], calldata[6])?;
        let execution_height = calldata[7].try_into().unwrap();

        Ok(EpochProof {
            header_root,
            state_root,
            n_signers,
            execution_hash,
            execution_height,
        })
    }
}

fn combine_to_fixed_bytes(high: Felt, low: Felt) -> Result<FixedBytes<32>, String> {
    let mut bytes = [0u8; 32];
    let high_bytes = high.to_bytes_le();
    let low_bytes = low.to_bytes_le();

    bytes[0..16].copy_from_slice(&high_bytes);
    bytes[16..32].copy_from_slice(&low_bytes);

    Ok(FixedBytes::from_slice(bytes.as_slice()))
}

#[derive(Debug)]
pub struct StarknetClient {
    account: Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
    // provider: Arc<JsonRpcClient<HttpTransport>>,
}

#[derive(Debug)]
pub enum StarknetError {
    ProviderError(ProviderError),
    AccountError(String),
    TransactionError(String),
    TimeoutError,
}

impl StarknetClient {
    pub async fn new(rpc_url: &str, address: &str, priv_key: &str) -> Result<Self, StarknetError> {
        let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url).unwrap()));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex(priv_key).unwrap(),
        ));
        let address = Felt::from_hex(address).unwrap();
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

    pub async fn deploy_contract(
        &self,
        init_data: ContractInitializationData,
        config: &BankaiConfig,
    ) -> Result<Felt, StarknetError> {
        let contract_artifact: SierraClass =
            serde_json::from_reader(std::fs::File::open(config.contract_path.clone()).unwrap())
                .unwrap();
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
            Err(e) => {
                error!("Deployment failed with error: {:#?}", e);
                Err(StarknetError::AccountError(format!(
                    "Deployment failed: {:#?}",
                    e
                )))
            }
        }
    }

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
                return Err(StarknetError::TransactionError(format!(
                    "TransactionExecutionError: {:#?}",
                    e
                )));
            }
        }
    }

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
            .await
            .map_err(StarknetError::ProviderError)?;
        println!("committee_hash: {:?}", committee_hash);
        Ok(committee_hash)
    }

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
            .await
            .map_err(StarknetError::ProviderError)?;
        println!("epoch_proof: {:?}", epoch_proof);
        Ok(EpochProof::from_contract_return_value(epoch_proof).unwrap())
    }

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
            .await
            .map_err(StarknetError::ProviderError)?;
        Ok(*latest_epoch.first().unwrap())
    }

    // Computes the slot numbers for the current term.
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
            .await
            .map_err(StarknetError::ProviderError)?;
        println!("latest_committee_id: {:?}", latest_committee_id);
        Ok(*latest_committee_id.first().unwrap())
    }

    pub async fn wait_for_confirmation(&self, tx_hash: Felt) -> Result<(), StarknetError> {
        let max_retries = 20;
        let delay = Duration::from_secs(5);

        for _ in 0..max_retries {
            let status = self.get_transaction_status(tx_hash).await.unwrap();

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
                    // Still pending, wait and retry
                    sleep(delay).await;
                }
            }
        }

        Err(StarknetError::TimeoutError)
    }

    pub async fn get_transaction_status(
        &self,
        tx_hash: Felt,
    ) -> Result<TransactionStatus, StarknetError> {
        let provider = self.account.provider();
        let tx_status = provider
            .get_transaction_status(tx_hash)
            .await
            .map_err(StarknetError::ProviderError)?;

        Ok(tx_status)
    }
}

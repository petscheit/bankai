use starknet::accounts::{Account, ConnectedAccount};
use starknet::core::types::{Call, FunctionCall};
use starknet::macros::selector;
use starknet::providers::{Provider, ProviderError};
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    contract::ContractFactory,
    core::{
        chain_id,
        types::{contract::SierraClass, BlockId, BlockTag, Felt},
    },
    macros::felt,
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Url,
    },
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;

use crate::contract_init::ContractInitializationData;
use crate::traits::Submittable;
use crate::BankaiConfig;
pub struct StarknetClient {
    account: Arc<SingleOwnerAccount<JsonRpcClient<HttpTransport>, LocalWallet>>,
    // provider: Arc<JsonRpcClient<HttpTransport>>,
}

#[derive(Debug)]
pub enum StarknetError {
    ProviderError(ProviderError),
    AccountError(String),
}

impl StarknetClient {
    pub async fn new(rpc_url: &str) -> Result<Self, StarknetError> {
        let provider = JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url).unwrap()));

        let signer = LocalWallet::from(SigningKey::from_secret_scalar(
            Felt::from_hex("00000000000000000000000000000000ed744265ce4c723fc93dc990842d0d3b")
                .unwrap(),
        ));
        let address =
            Felt::from_hex("46d40ee9ddf64f6a92b04f26902f67a76c93692b8637afd43daeeeebc836609")
                .unwrap();
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

        let contract_factory = ContractFactory::new(class_hash, self.account.clone());
        let deploy_tx = contract_factory.deploy_v1(init_data.to_calldata(), felt!("1122"), false);

        let contract_address = deploy_tx.deployed_address();

        assert!(
            contract_address == config.contract_address,
            "Contract address mismatch! Please update config: {:?}",
            contract_address
        );

        deploy_tx
            .send()
            .await
            .map_err(|e| StarknetError::AccountError(e.to_string()))?;

        Ok(contract_address)
    }

    pub async fn submit_update<T>(
        &self,
        update: impl Submittable<T>,
        config: &BankaiConfig,
    ) -> Result<(), StarknetError> {
        let result = self
            .account
            .execute_v1(vec![Call {
                to: config.contract_address,
                selector: update.get_contract_selector(),
                calldata: update.to_calldata(),
            }])
            .send()
            .await
            .map_err(|e| StarknetError::AccountError(e.to_string()))?;

        println!("tx_hash: {:?}", result.transaction_hash);
        Ok(())
    }

    pub async fn get_committee_hash(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<(), StarknetError> {
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
        Ok(())
    }

    pub async fn get_epoch_proof(
        &self,
        slot: u64,
        config: &BankaiConfig,
    ) -> Result<(), StarknetError> {
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
        Ok(())
    }

    pub async fn get_latest_epoch(&self, config: &BankaiConfig) -> Result<Felt, StarknetError> {
        let latest_epoch = self
            .account
            .provider()
            .call(
                FunctionCall {
                    contract_address: config.contract_address,
                    entry_point_selector: selector!("get_latest_epoch"),
                    calldata: vec![],
                },
                BlockId::Tag(BlockTag::Latest),
            )
            .await
            .map_err(StarknetError::ProviderError)?;
        Ok(*latest_epoch.first().unwrap())
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
}

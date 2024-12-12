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

// Dynamic:
// {
//   "program_hash": "0x61c9a8dc4629396452bffa605c59c947a4a344d85c6496f591787f2b6c422db",
//   "output": [
//     "0x5d0311ddd386651921936a0b7373eb78",
//     "0xe9860b10fbbaba73406d75c730de134d",
//     "0xf11e20d00175aa256439064009c9c462",
//     "0xe230206d9ff801787ee0a8c9162d7846",
//     "0x8a92568013993c8ed740b92bd5793883",
//     "0x4bff5824ce30177dca473afdeb840538",
//     "0x1e0",
//     "0x62a001"
//   ],
//   "bootloader_output": [
//     "0x1",
//     "0xa",
//     "0x61c9a8dc4629396452bffa605c59c947a4a344d85c6496f591787f2b6c422db",
//     "0x5d0311ddd386651921936a0b7373eb78",
//     "0xe9860b10fbbaba73406d75c730de134d",
//     "0xf11e20d00175aa256439064009c9c462",
//     "0xe230206d9ff801787ee0a8c9162d7846",
//     "0x8a92568013993c8ed740b92bd5793883",
//     "0x4bff5824ce30177dca473afdeb840538",
//     "0x1e0",
//     "0x62a001"
//   ],
//   "bootloader_output_hash": "0x553fdcc1e5a5478deb21396fc5d6c05d53c981bab7080d7198db72ebbec5f5c",
//   "bootloader_program_hash": "0x5ab580b04e3532b6b18f81cfa654a05e29dd8e2352d88df1e765a84072db07",
//   "fact_hash": "0x87393846ed7a88f1c49b21131e142e94c24f257451fd54f3ea84cc6c75c2b7"
// }

// Wrapped:
// {
//   "program_hash": "0x193641eb151b0f41674641089952e60bc3aded26e3cf42793655c562b8c3aa0",
//   "output": [
//     "0x5ab580b04e3532b6b18f81cfa654a05e29dd8e2352d88df1e765a84072db07",
//     "0x553fdcc1e5a5478deb21396fc5d6c05d53c981bab7080d7198db72ebbec5f5c"
//   ],
//   "bootloader_output": [
//     "0x1",
//     "0x4",
//     "0x193641eb151b0f41674641089952e60bc3aded26e3cf42793655c562b8c3aa0",
//     "0x5ab580b04e3532b6b18f81cfa654a05e29dd8e2352d88df1e765a84072db07",
//     "0x553fdcc1e5a5478deb21396fc5d6c05d53c981bab7080d7198db72ebbec5f5c"
//   ],
//   "bootloader_output_hash": "0x53e5be3155eee670c5ac9bb19f3963e95a6c54a9b2abb0c189ef231e96c8c55",
//   "bootloader_program_hash": "0x5ab580b04e3532b6b18f81cfa654a05e29dd8e2352d88df1e765a84072db07",
//   "fact_hash": "0x6eb3f948e3dae93777e01b0a2f16caf6074a8b97ae328f77819e135c0892bed"
// }

// fn calculate_wrapped_bootloaded_fact_hash(
//     wrapper_program_hash: felt252, bootloader_program_hash: felt252, child_program_hash: felt252, child_output: Span<felt252>
// ) -> felt252 {
//     let mut bootloader_output = PoseidonImpl::new()
//         .update(0x1)
//         .update(child_output.len().into() + 2)
//         .update(child_program_hash);
//     for x in child_output {
//         bootloader_output = bootloader_output.update(*x);
//     };

//     let mut wrapper_output = PoseidonImpl::new()
//         .update(0x1)
//         .update(0x4)
//         .update(wrapper_program_hash)
//         .update(bootloader_program_hash)
//         .update(bootloader_output.finalize());

//     PoseidonImpl::new()
//         .update(bootloader_program_hash)
//         .update(bootloader_output.finalize())
//         .finalize()
// }

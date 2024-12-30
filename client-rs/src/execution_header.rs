use crate::Error;
use crate::utils::rpc::BeaconRpcClient;
use beacon_state_proof::{rpc::fetch_beacon_state};
use beacon_state_proof::state_proof_fetcher::{TreeHash};

use types::{ExecutionPayload, ExecutionPayloadHeader, ExecutionPayloadHeaderRef, MainnetEthSpec};
pub struct ExecutionHeaderProof();

impl ExecutionHeaderProof {
    pub async fn fetch_proof(client: &BeaconRpcClient, slot: u64) -> Result<(), Error> {
        let beacon_block_body: types::BeaconBlockBody<MainnetEthSpec> = client.get_block_body(slot).await?;

        let root = beacon_block_body.tree_hash_root();
        println!("Root: {:?}", root);

        // let payload: ExecutionPayload<MainnetEthSpec> = beacon_block_body.execution_payload().unwrap().into();

        // // println!("{:?}", payload);


        // let tx_root = payload.transactions().merkle_root();

        // println!("Tx root: {:?}", tx_root);

        Ok(())
    }
}

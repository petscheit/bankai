use crate::Error;
use crate::utils::rpc::BeaconRpcClient;
use alloy_primitives::FixedBytes;
use beacon_state_proof::state_proof_fetcher::TreeHash;
use crate::utils::merkle::{hash_merkle_path, generate_merkle_path};
use types::{BeaconBlockBody, ExecPayload, ExecutionPayloadHeader, MainnetEthSpec};
use serde::{Serialize, Deserialize};
const EXECUTION_PAYLOAD_LEAF_INDEX: usize = 9;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionHeaderProof {
    pub root: FixedBytes<32>,
    pub path: Vec<FixedBytes<32>>,
    pub leaf: FixedBytes<32>,
    pub index: usize,
    pub execution_payload_header: ExecutionPayloadHeader<MainnetEthSpec>,
    pub slot: u64,
}

impl ExecutionHeaderProof {
    pub async fn fetch_proof(client: &BeaconRpcClient, slot: u64) -> Result<ExecutionHeaderProof, Error> {
        let beacon_block_body: BeaconBlockBody<MainnetEthSpec> = client.get_block_body(slot).await?;
        let root = beacon_block_body.tree_hash_root();

        let payload = beacon_block_body.execution_payload().unwrap().to_execution_payload_header();
        println!("{:#?}", payload);

        let body_ref = beacon_block_body.to_ref();
        let leafs: Vec<FixedBytes<32>> = body_ref.body_merkle_leaves().into_iter().map(|leaf| FixedBytes::from_slice(leaf.as_bytes())).collect();
        let path = generate_merkle_path(leafs.clone(), 9).unwrap();

        // ToDo: Facepalm
        // let other_path = beacon_block_body.block_body_merkle_proof(25).unwrap();
        // println!("Other path: {:?}", other_path);
        
        let leaf = leafs[9];
        
        let computed_root = hash_merkle_path(path.clone(), leaf.clone(), 9);        
        assert_eq!(computed_root.as_slice(), root.as_bytes());

        let proof = ExecutionHeaderProof {
            root: FixedBytes::from_slice(root.as_bytes()),
            path: path,
            leaf: leaf,

            index: 9,
            execution_payload_header: payload,
            slot,
        };

        Ok(proof)
        // Concatenate all leaves into a single bytes array
        // let mut bytes = Vec::new();

        // for leaf in leafs {
        //     bytes.extend_from_slice(&leaf.as_bytes());
        // }
        // let tree_hash = tree_hash::merkle_root(&bytes, 0);
        // println!("Tree hash: {:?}", tree_hash);
    }

}
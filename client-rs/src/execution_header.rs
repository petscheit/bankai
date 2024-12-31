use crate::Error;
use crate::utils::rpc::BeaconRpcClient;
use alloy_primitives::FixedBytes;
use beacon_state_proof::state_proof_fetcher::TreeHash;
use crate::utils::merkle::{hash_merkle_path, generate_merkle_path};
use types::{BeaconBlockBody, MainnetEthSpec};

const EXECUTION_PAYLOAD_LEAF_INDEX: usize = 9;
pub struct ExecutionHeaderProof();

impl ExecutionHeaderProof {
    pub async fn fetch_proof(client: &BeaconRpcClient, slot: u64) -> Result<(), Error> {
        let beacon_block_body: BeaconBlockBody<MainnetEthSpec> = client.get_block_body(slot).await?;

        // let blinded_beacon_block_body = beacon_block_body.clone_as_blinded();

        let root = beacon_block_body.tree_hash_root();
        println!("Root: {:?}", root);

        let payload = beacon_block_body.execution_payload().unwrap();
        let payload_root = payload.tree_hash_root();
        println!("Payload root: {:?}", payload_root);

        // let blinded_payload = payload.blinded_payload();

        
        let body_ref = beacon_block_body.to_ref();
        let leafs: Vec<FixedBytes<32>> = body_ref.body_merkle_leaves().into_iter().map(|leaf| FixedBytes::from_slice(leaf.as_bytes())).collect();

        
        let path = generate_merkle_path(leafs.clone(), 9).unwrap();
        println!("Path: {:?}", path);

        // ToDo: Facepalm
        // let other_path = beacon_block_body.block_body_merkle_proof(25).unwrap();
        // println!("Other path: {:?}", other_path);
        
        let leaf = leafs[9];
        println!("Leaf: {:?}", leaf);
        
        let computed_root = hash_merkle_path(path, leaf, 9);
        println!("Computed root: {:?}", computed_root);
        
        assert_eq!(computed_root.as_slice(), root.as_bytes());

        // Concatenate all leaves into a single bytes array
        // let mut bytes = Vec::new();

        // for leaf in leafs {
        //     bytes.extend_from_slice(&leaf.as_bytes());
        // }
        // let tree_hash = tree_hash::merkle_root(&bytes, 0);
        // println!("Tree hash: {:?}", tree_hash);

        Ok(())
    }

}
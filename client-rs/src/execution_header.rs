use crate::Error;
use crate::utils::rpc::BeaconRpcClient;
use alloy_primitives::FixedBytes;
use beacon_state_proof::{rpc::fetch_beacon_state};
use beacon_state_proof::state_proof_fetcher::{TreeHash};
use tree_hash;
use crate::utils::merkle::hash_merkle_path;
use ssz::Encode;
// use ssz::encode::Encode;
use ssz_derive::Encode;
use types::{BeaconBlockBody, MainnetEthSpec};
use sha2::{Digest, Sha256};

pub struct ExecutionHeaderProof();

impl ExecutionHeaderProof {
    pub async fn fetch_proof(client: &BeaconRpcClient, slot: u64) -> Result<(), Error> {
        let beacon_block_body: BeaconBlockBody<MainnetEthSpec> = client.get_block_body(slot).await?;
        let root = beacon_block_body.tree_hash_root();
        println!("Root: {:?}", root);
        
        let body_ref = beacon_block_body.to_ref();
        let leafs = body_ref.body_merkle_leaves();
        println!("Bytes: {:?}", leafs);

        // Concatenate all leaves into a single bytes array
        // let mut bytes = Vec::new();

        // for leaf in leafs {
        //     bytes.extend_from_slice(&leaf.as_bytes());
        // }
        // let tree_hash = tree_hash::merkle_root(&bytes, 0);
        // println!("Tree hash: {:?}", tree_hash);

        let leaf = FixedBytes::from_slice(leafs[4].as_bytes());
        let converted_leafs: Vec<Vec<u8>> = leafs.into_iter().map(|leaf| leaf.as_bytes().to_vec()).collect();

        let converted_leaf_refs: Vec<&[u8]> = converted_leafs.iter().map(|v| v.as_slice()).collect();
        let mut path = ExecutionHeaderProof::generate_merkle_path(converted_leaf_refs, 4).unwrap();
        println!("Path: {:?}", path);


        let computed_root = hash_merkle_path(path, leaf, 4);
        println!("Computed root: {:?}", computed_root);

        assert_eq!(computed_root.as_slice(), root.as_bytes());

        Ok(())
    }

    pub fn generate_merkle_path(leaves: Vec<&[u8]>, leaf_index: usize) -> Result<Vec<FixedBytes<32>>, Error> {
        if leaf_index >= leaves.len() {
            panic!("Leaf index out of bounds");
        }

        // Calculate the smallest power of 2 that can fit all leaves
        let mut tree_size = 1;
        while tree_size < leaves.len() {
            tree_size *= 2;
        }

        let mut path = Vec::new();
        let mut current_level: Vec<[u8; 32]> = leaves
            .iter()
            .map(|leaf| {
                let mut array = [0u8; 32];
                array.copy_from_slice(leaf);
                array
            })
            .collect();

        // Pad with zero hashes to reach the next power of 2
        while current_level.len() < tree_size {
            current_level.push([0u8; 32]);
        }

        let mut current_index = leaf_index;

        // Generate proof up to root
        while current_level.len() > 1 {
            let is_right = current_index % 2 == 1;
            let sibling_index = if is_right {
                current_index - 1
            } else {
                current_index + 1
            };

            // Add sibling to proof
            path.push(FixedBytes::from_slice(&current_level[sibling_index]));

            // Prepare next level
            let mut next_level = Vec::with_capacity(current_level.len() / 2);
            for pair in current_level.chunks(2) {
                let mut data = [0u8; 64];
                data[0..32].copy_from_slice(&pair[0]);
                data[32..64].copy_from_slice(&pair[1]);
                let hash = Sha256::digest(data);
                let mut result = [0u8; 32];
                result.copy_from_slice(&hash);
                next_level.push(result);
            }

            current_level = next_level;
            current_index /= 2;
        }

        Ok(path)
    }
}

// trait MerkleProofs {
//     fn encode_leafs(&self);
// }

// pub struct BeaconBlockBodyWrapper<E: EthSpec>(BeaconBlockBody<E>);

// impl MerkleProofs for BeaconBlockBodyWrapper<MainnetEthSpec>
// {
//     fn encode_leafs(&self) {
//         let bytes = match &self.0 {
//             // BeaconBlockBody::Base(inner) => inner.as_ssz_bytes(),
//             BeaconBlockBody::Altair(inner) => inner.as_ssz_bytes(),
//             _ => panic!("Unsupported BeaconBlockBody type"),
//             // BeaconBlockBody::Bellatrix(inner) => ssz::ssz_encode(inner),
//             // BeaconBlockBody::Capella(inner) => ssz::ssz_encode(inner),
//             // BeaconBlockBody::Deneb(inner) => ssz::ssz_encode(inner),
//             // BeaconBlockBody::Electra(inner) => ssz::ssz_encode(inner),
//         };
//         println!("Bytes: {:?}", bytes);
//     }
// }
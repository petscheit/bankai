use crate::utils::merkle::sha256::{generate_path, hash_path};
// use crate::utils::rpc::BeaconRpcClient;
use crate::clients::beacon_chain::{BeaconError, BeaconRpcClient};
use alloy_primitives::FixedBytes;
use beacon_state_proof::state_proof_fetcher::TreeHash;
use beacon_types::{BeaconBlockBody, ExecPayload, ExecutionPayloadHeader, MainnetEthSpec};
use serde::{Deserialize, Serialize};
use thiserror::Error;
/// Index of the execution payload in the beacon block body merkle tree
const EXECUTION_PAYLOAD_LEAF_INDEX: usize = 9;

/// Represents a proof of inclusion for an execution payload header in a beacon block
///
/// This structure contains all necessary components to verify that an execution payload
/// header is part of a specific beacon block through merkle proof verification.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionHeaderProof {
    /// Root hash of the beacon block body merkle tree
    pub root: FixedBytes<32>,
    /// Merkle proof path containing the intermediate hashes
    pub path: Vec<FixedBytes<32>>,
    /// Hash of the execution payload header (leaf node)
    pub leaf: FixedBytes<32>,
    /// Position of the execution payload in the merkle tree. Should be 9.
    pub index: usize,
    /// The actual execution payload header data
    pub execution_payload_header: ExecutionPayloadHeader<MainnetEthSpec>,
    /// Slot number of the beacon block containing this payload
    pub slot: u64,
}

impl ExecutionHeaderProof {
    /// Fetches and constructs a merkle proof for an execution payload header at a given slot
    ///
    /// # Arguments
    /// * `client` - Reference to the beacon node RPC client
    /// * `slot` - The slot number to fetch the proof for
    ///
    /// # Returns
    /// * `Result<ExecutionHeaderProof, Error>` - The constructed proof or an error
    /// ```
    pub async fn fetch_proof(
        client: &BeaconRpcClient,
        slot: u64,
    ) -> Result<ExecutionHeaderProof, ExecutionHeaderError> {
        // Fetch the beacon block body for the specified slot
        let beacon_block_body: BeaconBlockBody<MainnetEthSpec> =
            client.get_block_body(slot).await?;
        let root = beacon_block_body.tree_hash_root();

        // Extract the execution payload header
        let payload: ExecutionPayloadHeader<MainnetEthSpec> = beacon_block_body
            .execution_payload()
            .unwrap()
            .to_execution_payload_header();

        // Generate merkle proof components
        let body_ref = beacon_block_body.to_ref();
        let leafs: Vec<FixedBytes<32>> = body_ref
            .body_merkle_leaves()
            .into_iter()
            .map(|leaf| FixedBytes::from_slice(leaf.as_slice()))
            .collect();

        let path = generate_path(leafs.clone(), EXECUTION_PAYLOAD_LEAF_INDEX).unwrap();
        let leaf = leafs[EXECUTION_PAYLOAD_LEAF_INDEX];

        // Verify the merkle proof
        let computed_root = hash_path(path.clone(), leaf, EXECUTION_PAYLOAD_LEAF_INDEX as u64);
        assert_eq!(computed_root.as_slice(), root.as_slice());

        // Construct and return the proof
        let proof = ExecutionHeaderProof {
            root: FixedBytes::from_slice(root.as_slice()),
            path,
            leaf,
            index: EXECUTION_PAYLOAD_LEAF_INDEX,
            execution_payload_header: payload,
            slot,
        };

        Ok(proof)
    }
}

#[derive(Debug, Error)]
pub enum ExecutionHeaderError {
    #[error("Beacon error: {0}")]
    Beacon(#[from] BeaconError),
}

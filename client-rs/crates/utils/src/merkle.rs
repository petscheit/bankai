use serde::{Deserialize, Serialize};
use starknet::core::types::Felt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePath {
    pub leaf_index: u64,
    pub value: Felt,
}

pub(crate) mod sha256 {
    use types::error::Error;
    use alloy_primitives::FixedBytes;
    use sha2::{Digest, Sha256};

    pub fn hash_path(
        path: Vec<FixedBytes<32>>,
        leaf: FixedBytes<32>,
        index: u64,
    ) -> FixedBytes<32> {
        let mut value = leaf;
        let mut data = [0u8; 64];
        let mut g_index = index;
        let mut witness = path.into_iter().rev().collect::<Vec<FixedBytes<32>>>();

        while let Some(sibling) = witness.pop() {
            if g_index % 2 == 0 {
                // left node
                data[0..32].copy_from_slice(value.as_slice());
                data[32..64].copy_from_slice(sibling.as_slice());
            } else {
                data[0..32].copy_from_slice(sibling.as_slice());
                data[32..64].copy_from_slice(value.as_slice());
            }
            value = FixedBytes::from_slice(&Sha256::digest(data));
            g_index /= 2;
        }
        value
    }

    pub fn generate_path(
        leaves: Vec<FixedBytes<32>>,
        leaf_index: usize,
    ) -> Result<Vec<FixedBytes<32>>, Error> {
        if leaf_index >= leaves.len() {
            return Err(Error::InvalidMerkleTree);
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
                array.copy_from_slice(leaf.as_slice());
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

pub(crate) mod poseidon {
    use starknet_crypto::{poseidon_hash, Felt};

    pub fn compute_root(leaves: Vec<Felt>) -> Felt {
        // Calculate the smallest power of 2 that can fit all leaves
        let mut tree_size = 1;
        while tree_size < leaves.len() {
            tree_size *= 2;
        }

        let mut current_level = leaves;

        // Pad with zero hashes to reach the next power of 2
        while current_level.len() < tree_size {
            current_level.push(Felt::ZERO);
        }

        // Build tree level by level until we reach the root
        while current_level.len() > 1 {
            let mut next_level = Vec::with_capacity(current_level.len() / 2);

            // Process pairs of nodes
            for pair in current_level.chunks(2) {
                let hash = poseidon_hash(pair[0], pair[1]);
                next_level.push(hash);
            }

            current_level = next_level;
        }

        // Return the root (the only remaining element)
        current_level[0]
    }

    pub fn compute_paths(leaves: Vec<Felt>) -> (Felt, Vec<Vec<Felt>>) {
        // Calculate the smallest power of 2 that can fit all leaves
        let mut tree_size = 1;
        while tree_size < leaves.len() {
            tree_size *= 2;
        }

        let mut current_level = leaves.clone();

        // Pad with zero hashes to reach the next power of 2
        while current_level.len() < tree_size {
            current_level.push(Felt::ZERO);
        }

        // Store all levels of the tree to construct paths later
        let mut tree_levels = vec![current_level.clone()];

        // Build tree level by level until we reach the root
        while current_level.len() > 1 {
            let mut next_level = Vec::with_capacity(current_level.len() / 2);

            // Process pairs of nodes
            for pair in current_level.chunks(2) {
                let hash = poseidon_hash(pair[0], pair[1]);
                next_level.push(hash);
            }

            current_level = next_level;
            tree_levels.push(current_level.clone());
        }

        let root = tree_levels.last().unwrap()[0];

        // Generate a path for each original leaf
        let mut paths = Vec::with_capacity(leaves.len());

        for leaf_idx in 0..leaves.len() {
            let mut path = Vec::new();
            let mut current_idx = leaf_idx;

            // Go through each level (except the root) to build the path
            for level in &tree_levels[..tree_levels.len() - 1] {
                // If current_idx is even, take the right sibling
                // If current_idx is odd, take the left sibling
                let sibling_idx = if current_idx % 2 == 0 {
                    current_idx + 1
                } else {
                    current_idx - 1
                };

                path.push(level[sibling_idx]);
                current_idx /= 2;
            }

            paths.push(path);
        }

        (root, paths)
    }

    pub fn hash_path(leaf: Felt, path: &[Felt], index: usize) -> Felt {
        let mut current_hash = leaf;
        let mut current_index = index;

        // Walk up the tree using the path
        for sibling in path {
            // Determine if the current_hash is the left or right node
            let (left, right) = if current_index % 2 == 0 {
                (current_hash, *sibling)
            } else {
                (*sibling, current_hash)
            };

            // Hash the pair to get the parent
            current_hash = poseidon_hash(left, right);
            current_index /= 2;
        }

        current_hash
    }
}

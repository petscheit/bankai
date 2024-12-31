use alloy_primitives::FixedBytes;
use sha2::{Digest, Sha256};
use crate::Error;

pub fn hash_merkle_path(
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

pub fn generate_merkle_path(leaves: Vec<FixedBytes<32>>, leaf_index: usize) -> Result<Vec<FixedBytes<32>>, Error> {
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

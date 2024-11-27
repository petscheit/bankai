use alloy_primitives::FixedBytes;
use sha2::{Digest, Sha256};

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
        value = FixedBytes::from_slice(&Sha256::digest(&data));
        g_index /= 2;
    }
    value
}

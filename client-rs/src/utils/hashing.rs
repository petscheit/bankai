use alloy_primitives::FixedBytes;
use bls12_381::G1Affine;
use sha2::{Digest, Sha256};

pub fn get_committee_hash(point: G1Affine) -> FixedBytes<32> {
    let mut hasher = Sha256::new();
    let uncompressed = point.to_uncompressed();
    hasher.update(uncompressed.as_ref());
    FixedBytes::from_slice(&hasher.finalize())
}

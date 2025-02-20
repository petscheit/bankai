use core::poseidon::PoseidonImpl;
use core::poseidon::hades_permutation;
use core::hash::HashStateTrait;
use core::poseidon::poseidon_hash_span;

pub const WRAPPER_PROGRAM_HASH: felt252 =
    0x193641eb151b0f41674641089952e60bc3aded26e3cf42793655c562b8c3aa0;

pub fn calculate_wrapped_bootloaded_fact_hash(
    wrapper_program_hash: felt252,
    bootloader_program_hash: felt252,
    child_program_hash: felt252,
    child_output: Span<felt252>,
) -> felt252 {
    let mut bootloader_output = PoseidonImpl::new()
        .update(0x1)
        .update(child_output.len().into() + 2)
        .update(child_program_hash);
    for x in child_output {
        bootloader_output = bootloader_output.update(*x);
    };

    let mut wrapper_output = PoseidonImpl::new()
        .update(0x1)
        .update(0x4)
        .update(wrapper_program_hash)
        .update(bootloader_program_hash)
        .update(bootloader_output.finalize());

    PoseidonImpl::new().update(bootloader_program_hash).update(wrapper_output.finalize()).finalize()
}

pub fn hash_path(leaf: felt252, path: Array<felt252>, index: u16) -> felt252 {
    // Initialize current hash and index
    let mut current_hash = leaf;
    let mut current_index = index;

    // Iterate through the path
    let mut i: usize = 0;
    loop {
        if i >= path.len() {
            break;
        }

        // Get the sibling node
        let sibling = *path.at(i);

        // Determine left and right nodes based on current_index's least significant bit
        let (left, right) = if (current_index & 1_u16 == 0_u16) {
            (current_hash, sibling)
        } else {
            (sibling, current_hash)
        };

        // Hash the pair using Poseidon
        let (hash, _, _) = hades_permutation(left, right, 2);
        current_hash = hash;

        // Update index for next level
        current_index = current_index / 2;
        i += 1;
    };

    current_hash
}

pub fn compute_leaf_hash(
    header_root: u256,
    beacon_state_root: u256,
    slot: u64,
    committee_hash: u256,
    n_signers: u64,
    execution_hash: u256,
    execution_height: u64,
) -> felt252 {
    // Convert all inputs to felt252 and create array
    let mut values = array![
        header_root.low.into(),
        header_root.high.into(),
        beacon_state_root.low.into(),
        beacon_state_root.high.into(),
        slot.into(),
        committee_hash.low.into(),
        committee_hash.high.into(),
        n_signers.into(),
        execution_hash.low.into(),
        execution_hash.high.into(),
        execution_height.into(),
    ];

    // Hash all values with Poseidon
    poseidon_hash_span(values.span())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_leaf_hash_computation() {
        // Test values from JSON file
        let header_root = 0xcee6e3a29b289c3d0eb1f08f6cbf965a2f5771f54ca781fbf1f9d9a5e898d602_u256;
        let beacon_state_root =
            0xac1d83f6ab8c04205b698f9b5dbe93a1136000ca0162941bf129029ad402906c_u256;
        let slot = 6710272_u64;
        let committee_hash =
            0x3ccf068854b1612cc9537f6fd2a56fb0734722ce40b89685f84e17a6986510d3_u256;
        let n_signers = 479_u64;
        let execution_hash =
            0xc2c133b1ea59352cef6c0434e0007cdba4bdc216afd32fdf6b40c4a135a8535e_u256;
        let execution_height = 7440225_u64;

        // Compute hash using our function
        let computed_hash = compute_leaf_hash(
            header_root,
            beacon_state_root,
            slot,
            committee_hash,
            n_signers,
            execution_hash,
            execution_height,
        );

        let expected_hash = 0xBA8230D3714675CA5E80A257F3F2F581959A5E474E40101C52153192FD7728;

        // Assert they match
        assert_eq!(computed_hash, expected_hash, "Leaf hash computation mismatch");
    }


    #[test]
    fn test_hash_path_verification() {
        // Same leaf data as previous test
        let header_root = 0xcee6e3a29b289c3d0eb1f08f6cbf965a2f5771f54ca781fbf1f9d9a5e898d602_u256;
        let beacon_state_root =
            0xac1d83f6ab8c04205b698f9b5dbe93a1136000ca0162941bf129029ad402906c_u256;
        let slot = 6710272_u64;
        let committee_hash =
            0x3ccf068854b1612cc9537f6fd2a56fb0734722ce40b89685f84e17a6986510d3_u256;
        let n_signers = 479_u64;
        let execution_hash =
            0xc2c133b1ea59352cef6c0434e0007cdba4bdc216afd32fdf6b40c4a135a8535e_u256;
        let execution_height = 7440225_u64;

        // Compute the leaf hash
        let leaf_hash = compute_leaf_hash(
            header_root,
            beacon_state_root,
            slot,
            committee_hash,
            n_signers,
            execution_hash,
            execution_height,
        );

        // Merkle path from JSON
        let path = array![
            0x0,
            0x293d3e8a80f400daaaffdd5932e2bcc8814bab8f414a75dcacf87318f8b14c5,
            0x296ec483967ad3fbe3407233db378b6284cc1fcc78d62457b97a4be6744ad0d,
            0x4127be83b42296fe28f98f8fdda29b96e22e5d90501f7d31b84e729ec2fac3f,
            0x33883305ab0df1ab7610153578a4d510b845841b84d90ed993133ce4ce8f827,
            0x6114fdf0455660a422ac813130104438c7baf332cc1eca0618957a3aeb68795,
        ];

        let index = 32_u16;

        // Compute root using hash_path
        let computed_root = hash_path(leaf_hash, path, index);

        // Expected root is the first value in the JSON array (0x0)
        let expected_root =
            3014209719831846118507369742452047831482182187060364606511726060971609846063;

        assert_eq!(computed_root, expected_root, "Merkle root computation mismatch");
    }
}

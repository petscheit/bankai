from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.alloc import alloc

// UNSAFE: hash a pair of uint256s using sha256
func unsafe_sha256(left: Uint256, right: Uint256) -> Uint256 {
    alloc_locals;

    local result: Uint256;
    %{
        import hashlib

        def uint256_to_hex(uint256):
            return f"{uint256.high:032x}{uint256.low:032x}"

        m = hashlib.sha256()
        left_hex = uint256_to_hex(ids.left)
        right_hex = uint256_to_hex(ids.right)
        m.update(bytes.fromhex(left_hex))
        m.update(bytes.fromhex(right_hex))
        pair_val = m.hexdigest()
        #print("pair_val: ", pair_val)
        res = int(pair_val, 16)
        ids.result.high, ids.result.low = divmod(res, 2**128)
    %}

    return result;
}

// UNSAFE: hash a list of uint256s using sha256
func unsafe_sha256_uint256s(values: Uint256*, n_values: felt) -> Uint256 {
    alloc_locals;

    local result: Uint256;
    %{
        import hashlib

        def uint256_to_hex(uint256):
            return f"{uint256.high:032x}{uint256.low:032x}"

        m = hashlib.sha256()
        i = 0
        while i < ids.n_values:
            m.update(bytes.fromhex(uint256_to_hex(ids.values[i])))
            i += 1
        # pad if odd number of values
        if ids.n_values % 2 == 1:
            m.update(bytes.fromhex(uint256_to_hex(ids.values[ids.n_values - 1])))
        pair_val = m.hexdigest()
        print("pair_val: ", pair_val)
        res = int(pair_val, 16)
        ids.result.high, ids.result.low = divmod(res, 2**128)
    %}

    return result;
}

// UNSAFE: compute the merkle root of a list of uint256s
func unsafe_compute_merkle_root(
    leafs: Uint256*, leafs_len: felt
) -> Uint256 {
    alloc_locals;
    
    // assert that leafs_len is a power of 2
    %{ assert (ids.leafs_len & (ids.leafs_len - 1)) == 0 %}


    if (leafs_len == 0) {
        // keccak(0)
        return (
            Uint256(low=0x6612f7b477d66591ff96a9e064bcc98a, high=0xbc36789e7a1e281436464229828f817d)
        );
    }

    let (tree: Uint256*) = alloc();
    let tree_len = 2 * leafs_len - 1;

    // copy the leafs to the end of the tree array
    memcpy(dst=tree + (tree_len - leafs_len) * Uint256.SIZE, src=leafs, len=leafs_len * Uint256.SIZE);
    
    compute_merkle_root_inner(tree=tree, tree_range=tree_len - leafs_len - 1, index=0);
    return (tree[0]);
}

// Implements the merkle tree building logic. This follows the unordered StandardMerkleTree implementation of OpenZeppelin
func compute_merkle_root_inner(tree: Uint256*, tree_range: felt, index: felt) {
    if (tree_range + 1 == index) {
        return ();
    }

    let left_idx = (tree_range - index) * 2 + 1;
    let right_idx = (tree_range - index) * 2 + 2;

    let node = unsafe_sha256(left=tree[left_idx], right=tree[right_idx]);
    // %{ print("node[", ids.tree_range - ids.index, "]: ", hex(ids.node.low), hex(ids.node.high)) %}

    assert tree[tree_range - index] = node;

    return compute_merkle_root_inner(tree=tree, tree_range=tree_range, index=index + 1);
}


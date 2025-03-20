from starkware.cairo.common.builtin_poseidon.poseidon import poseidon_hash
from starkware.cairo.common.cairo_builtins import PoseidonBuiltin
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.memcpy import memcpy

namespace PoseidonMerkleTree {
    func compute_root{
        range_check_ptr, poseidon_ptr: PoseidonBuiltin*, pow2_array: felt*
    }(leafs: felt*, leafs_len: felt) -> felt {
        alloc_locals;

        // It needs to be ensured that the leafs_len is a power of 2.

        let (tree: felt*) = alloc();
        let tree_len = 2 * leafs_len - 1;  // total nodes in the tree

        // copy the leafs to the end of the tree array
        memcpy(dst=tree + (tree_len - leafs_len), src=leafs, len=leafs_len);

        // Calculate number of internal nodes to process
        let internal_nodes = leafs_len - 1;

        // Set up initial pointers:
        // tree_ptr starts at the last pair of leaves
        let tree_ptr = tree + tree_len;
        // out_ptr starts where first set of hashes should be written
        let out_ptr = tree + internal_nodes;

        compute_merkle_root_inner_optimized(
            tree_ptr=tree_ptr,
            out_ptr=out_ptr,
            steps=internal_nodes
        );

        // The root will be at the first position of the array
        return [tree];
    }

    func compute_merkle_root_inner_optimized{
        range_check_ptr,
        poseidon_ptr: PoseidonBuiltin*
    }(
        tree_ptr: felt*,   // Points to where we read children for hashing
        out_ptr: felt*,    // Points to where we place the newly computed hash
        steps: felt        // Number of internal nodes to compute
    ) {
        alloc_locals;

        // Base case: no more internal nodes to compute
        if (steps == 0) {
            return ();
        }

        // Move read pointer back by 2 to get the pair to hash
        tempvar new_tree_ptr = tree_ptr - 2;

        // Hash the pair of nodes
        let (node) = poseidon_hash([new_tree_ptr], [new_tree_ptr + 1]);

        // Store result and move write pointer back by 1
        tempvar new_out_ptr = out_ptr - 1;
        assert [new_out_ptr] = node;

        // Continue with remaining nodes
        return compute_merkle_root_inner_optimized(
            tree_ptr=new_tree_ptr,
            out_ptr=new_out_ptr,
            steps=steps - 1
        );
    }
}
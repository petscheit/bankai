// %builtins output range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin

from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256, uint256_reverse_endian
from starkware.cairo.common.builtin_keccak.keccak import keccak_uint256s_bigend
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.builtin_poseidon.poseidon import (
    poseidon_hash,
    poseidon_hash_many,
)
from sha import SHA256
from debug import print_string, print_felt, print_felt_hex
from cairo.src.utils import pow2alloc128, felt_divmod
from cairo.src.types import ExecutionHeaderProof
namespace SSZ {
    func hash_pair_container{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(left: Uint256, right: Uint256) -> Uint256 {
        alloc_locals;

        let input = MerkleUtils.chunk_pair(left, right);
        let (result_chunks) = SHA256.hash_64(input=input - 16);
        let result = MerkleUtils.chunks_to_uint256(output=result_chunks);
        return result;
    }

    func hash_header_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(
        slot: Uint256,
        proposer_index: Uint256,
        parent_root: Uint256,
        state_root: Uint256,
        body_root: Uint256,
    ) -> Uint256 {
        alloc_locals;
        // For numbers, we need to reverse the endianness
        let (slot) = uint256_reverse_endian(num=slot);
        let (proposer_index) = uint256_reverse_endian(num=proposer_index);

        let (leafs: Uint256*) = alloc();
        assert leafs[0] = slot;
        assert leafs[1] = proposer_index;
        assert leafs[2] = parent_root;
        assert leafs[3] = state_root;
        assert leafs[4] = body_root;

        // we need to pad, to make sure the length is a power of 2
        // ToDo: we can add some precomputation here
        assert leafs[5] = Uint256(low=0, high=0);
        assert leafs[6] = Uint256(low=0, high=0);
        assert leafs[7] = Uint256(low=0, high=0);

        let result = MerkleTree.compute_root(leafs=leafs, leafs_len=8);

        return result;
    }

    func hash_execution_payload_header_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(payload_fields: Uint256*) -> (header_root: Uint256, header_hash: Uint256, header_height: felt) {
        let leaf_segments = cast(payload_fields, felt*);
        memset(dst=leaf_segments + 34, value=0, n=30);

        let leafs = cast(leaf_segments, Uint256*);
        let root = MerkleTree.compute_root(leafs=leafs, leafs_len=32);

        let (header_height) = uint256_reverse_endian(num=leafs[6]);

        return (root, leafs[12], header_height.low);
    }
}



namespace MerkleTree {
    func compute_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(leafs: Uint256*, leafs_len: felt) -> Uint256 {
        alloc_locals;

        // ensure we have a power of 2.
        // ToDo: we should automatically add padding leafs
        // %{ assert ids.leafs_len & (ids.leafs_len - 1) == 0 %}

        // chunk the leafs and write to leafs array
        let (chunked_leafs: felt*) = alloc();
        MerkleUtils.chunk_leafs{
            range_check_ptr=range_check_ptr, pow2_array=pow2_array, output_ptr=chunked_leafs
        }(leafs=leafs, leafs_len=leafs_len, index=0);

        // move the pointer to the start of the chunked leafs
        let chunked_leafs = chunked_leafs - leafs_len * 8;

        let (tree: felt*) = alloc();
        let tree_len = 2 * leafs_len - 1;  // number nodes in the tree (not accounting for chunking)

        // copy the leafs to the end of the tree array
        memcpy(dst=tree + (tree_len - leafs_len) * 8, src=chunked_leafs, len=leafs_len * 8);

        with sha256_ptr {
            let tree = tree + tree_len * 8;  // move the pointer to the end of the tree
            compute_merkle_root_inner{
                range_check_ptr=range_check_ptr,
                sha256_ptr=sha256_ptr,
                pow2_array=pow2_array,
                tree_ptr=tree,
            }(tree_range=tree_len - leafs_len - 1, index=0);
        }

        let result = MerkleUtils.chunks_to_uint256(output=tree - 8);
        return result;
    }

    // Implements the merkle tree building logic. This follows the unordered StandardMerkleTree implementation of OpenZeppelin
    func compute_merkle_root_inner{
        range_check_ptr, sha256_ptr: felt*, pow2_array: felt*, tree_ptr: felt*
    }(tree_range: felt, index: felt) {
        alloc_locals;

        if (tree_range + 1 == index) {
            return ();
        }

        // for each iteration, we must move the pointer 16 felts back to the next pair
        tempvar tree_ptr = tree_ptr - 16;
        let (node) = SHA256.hash_64(input=tree_ptr);

        // write the hash to the correct position in the tree
        memcpy(dst=tree_ptr - (1 + tree_range - index) * 8, src=node, len=8);
        return compute_merkle_root_inner(tree_range=tree_range, index=index + 1);
    }

    func hash_merkle_path{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(path: felt**, path_len: felt, leaf: felt*, index: felt) -> Uint256 {
        alloc_locals;

        // Base case - if no more siblings to process, return the current value
        if (path_len == 0) {
            let result = MerkleUtils.chunks_to_uint256(output=leaf);
            return result;
        }

        // Check if current node is left or right child
        let (new_index, r) = felt_divmod(index, 2);
        if (r == 0) {
            // for some reason this break if I append to leaf, instead of doing this
            let (input: felt*) = alloc();
            memcpy(dst=input, src=leaf, len=8);
            memcpy(dst=input + 8, src=[path], len=8);
            let (result_chunks) = SHA256.hash_64(input=input);
        } else {
            memcpy(dst=[path] + 8, src=leaf, len=8);
            let (result_chunks) = SHA256.hash_64(input=[path]);
        }

        // Recurse with remaining path
        return hash_merkle_path(
            path=path + 1, path_len=path_len - 1, leaf=result_chunks, index=new_index
        );
    }
}

namespace MerkleUtils {
    func chunk_pair{range_check_ptr, pow2_array: felt*}(left: Uint256, right: Uint256) -> felt* {
        let (leafs: Uint256*) = alloc();
        assert leafs[0] = left;
        assert leafs[1] = right;

        let (output_ptr: felt*) = alloc();
        with output_ptr {
            chunk_leafs(leafs=leafs, leafs_len=2, index=0);
        }
        return output_ptr;
    }

    func chunk_leafs{range_check_ptr, pow2_array: felt*, output_ptr: felt*}(
        leafs: Uint256*, leafs_len: felt, index: felt
    ) {
        if (index == leafs_len) {
            return ();
        }

        let leaf = [leafs];

        // Process left-high
        let (q0, r0) = felt_divmod(leaf.high, pow2_array[32]);
        let (q1, r1) = felt_divmod(q0, pow2_array[32]);
        let (q2, r2) = felt_divmod(q1, pow2_array[32]);
        let (q3, r3) = felt_divmod(q2, pow2_array[32]);
        assert [output_ptr] = r3;
        assert [output_ptr + 1] = r2;
        assert [output_ptr + 2] = r1;
        assert [output_ptr + 3] = r0;

        // Proccess left-low
        let (q4, r4) = felt_divmod(leaf.low, pow2_array[32]);
        let (q5, r5) = felt_divmod(q4, pow2_array[32]);
        let (q6, r6) = felt_divmod(q5, pow2_array[32]);
        let (q7, r7) = felt_divmod(q6, pow2_array[32]);
        assert [output_ptr + 4] = r7;
        assert [output_ptr + 5] = r6;
        assert [output_ptr + 6] = r5;
        assert [output_ptr + 7] = r4;

        tempvar output_ptr = output_ptr + 8;
        return chunk_leafs(leafs=leafs + Uint256.SIZE, leafs_len=leafs_len, index=index + 1);
    }

    func chunk_uint256{range_check_ptr, pow2_array: felt*}(value: Uint256) -> felt* {
        let (output_ptr: felt*) = alloc();
        // Process left-high
        let (q0, r0) = felt_divmod(value.high, pow2_array[32]);
        let (q1, r1) = felt_divmod(q0, pow2_array[32]);
        let (q2, r2) = felt_divmod(q1, pow2_array[32]);
        let (q3, r3) = felt_divmod(q2, pow2_array[32]);
        assert [output_ptr] = r3;
        assert [output_ptr + 1] = r2;
        assert [output_ptr + 2] = r1;
        assert [output_ptr + 3] = r0;

        // Proccess left-low
        let (q4, r4) = felt_divmod(value.low, pow2_array[32]);
        let (q5, r5) = felt_divmod(q4, pow2_array[32]);
        let (q6, r6) = felt_divmod(q5, pow2_array[32]);
        let (q7, r7) = felt_divmod(q6, pow2_array[32]);
        assert [output_ptr + 4] = r7;
        assert [output_ptr + 5] = r6;
        assert [output_ptr + 6] = r5;
        assert [output_ptr + 7] = r4;

        return output_ptr;
    }

    func chunks_to_uint256{pow2_array: felt*}(output: felt*) -> Uint256 {
        let low = [output + 4] * pow2_array[96] + [output + 5] * pow2_array[64] + [output + 6] *
            pow2_array[32] + [output + 7];
        let high = [output] * pow2_array[96] + [output + 1] * pow2_array[64] + [output + 2] *
            pow2_array[32] + [output + 3];
        return (Uint256(low=low, high=high));
    }
}


// func main{
//     output_ptr: felt*,
//     range_check_ptr,
//     bitwise_ptr: BitwiseBuiltin*,
//     poseidon_ptr: PoseidonBuiltin*,
//     range_check96_ptr: felt*,
//     add_mod_ptr: ModBuiltin*,
//     mul_mod_ptr: ModBuiltin*,
// }() {
//     alloc_locals;

//     let (pow2_array) = pow2alloc128();
//     let (sha256_ptr, sha256_ptr_start) = SHA256.init();

//     with pow2_array, sha256_ptr {
//         SSZ.hash_execution_payload_header_root();
//     }

//     SHA256.finalize(sha256_ptr_start, sha256_ptr);

//     return ();
// }
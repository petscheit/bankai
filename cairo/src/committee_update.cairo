%builtins output range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin
from starkware.cairo.common.bitwise import bitwise_and
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.memset import memset
from definitions import UInt384

from cairo.src.utils import pow2alloc128, felt_divmod
from cairo.src.signer import commit_committee_key
from cairo.src.ssz import MerkleTree
from sha import SHA256, HashUtils
from ec_ops import derive_g1_point_from_x

// Main function to update the committee
func main{
    output_ptr: felt*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;

    // Allocate memory and initialize SHA256
    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();

    // Allocate memory for committee keys and path
    let (committee_keys_root: felt*) = alloc();
    let (path: felt**) = alloc();
    local path_len: felt;
    local aggregate_committee_key: UInt384;
    local slot: felt;

    // Initialize variables from program input
    %{
        from cairo.py.utils import write_uint384, hex_to_chunks_32, print_u256
        write_uint384(ids.aggregate_committee_key, int(program_input["next_aggregate_sync_committee"], 16))
        committee_keys_root = hex_to_chunks_32(program_input["committee_keys_root"])
        segments.write_arg(ids.committee_keys_root, committee_keys_root)
        ids.slot = int(program_input["beacon_slot"], 16)
        path = [hex_to_chunks_32(node) for node in program_input["next_sync_committee_branch"]]
        ids.path_len = len(path)
        segments.write_arg(ids.path, path)
    %}

    // Compute hashes and update state
    with sha256_ptr, pow2_array {
        let leaf_hash = compute_leaf_hash(committee_keys_root, aggregate_committee_key);
        // The next sync committee is always at index 55
        let state_root = MerkleTree.hash_merkle_path(path=path, path_len=path_len, leaf=leaf_hash, index=55);
        %{ print_u256("Derived state root", ids.state_root) %}
        let committee_hash = compute_committee_hash(aggregate_committee_key);

    }
    %{ print_u256("Derived committee hash", ids.committee_hash) %}

    // Finalize SHA256 and write output
    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);

    assert [output_ptr] = state_root.low;
    assert [output_ptr + 1] = state_root.high;
    assert [output_ptr + 2] = committee_hash.low;
    assert [output_ptr + 3] = committee_hash.high;
    assert [output_ptr + 4] = slot;
    let output_ptr = output_ptr + 5;

    return ();
}

// Compute the leaf hash for the Merkle tree
func compute_leaf_hash{
    range_check_ptr,
    pow2_array: felt*,
    sha256_ptr: felt*
}(committee_keys_root: felt*, aggregate_committee_key: UInt384) -> felt* {
    alloc_locals;
    // Step 1: Create leaf hash -> h(sync_committee_root, aggregate_committee_key)
    let (aggregate_committee_key_chunks) = HashUtils.chunk_uint384(aggregate_committee_key);
    // Pad the key to 64 bytes
    memset(dst=aggregate_committee_key_chunks + 12, value=0, n=4);
    let (aggregate_committee_root) = SHA256.hash_bytes(aggregate_committee_key_chunks, 64);

    // Copy the root and compute the final leaf hash
    memcpy(dst=committee_keys_root + 8, src=aggregate_committee_root, len=8);
    let (leaf_hash) = SHA256.hash_bytes(committee_keys_root, 64);
    return leaf_hash;
}

// Compute the hash of the committee point h(x||y)
func compute_committee_hash{
    range_check_ptr,
    sha256_ptr: felt*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    pow2_array: felt*,
}(compressed_g1: UInt384) -> Uint256 {
    alloc_locals;
    
    // Decompress G1 point and perform sanity checks
    let (flags, x_point) = decompress_g1(compressed_g1);
    assert flags.compression_bit = 1;
    assert flags.infinity_bit = 0;
    let s = UInt384(d0=flags.sign_bit, d1=0, d2=0, d3=0);

    // Derive the full G1 point and hash it
    let (point) = derive_g1_point_from_x(curve_id=1, x=x_point, s=s);
    let committee_hash = commit_committee_key(point=point);

    return committee_hash;
}

// Structure to hold flags for compressed G1 points
struct CompressedG1Flags {
    compression_bit: felt,  // Bit 383
    infinity_bit: felt,     // Bit 382
    sign_bit: felt,         // Bit 381
}

// Decompress a G1 point from its compressed form
func decompress_g1{
    range_check_ptr,
}(compressed_g1: UInt384) -> (CompressedG1Flags, UInt384) {
    alloc_locals;

    let limb = compressed_g1.d3;

    // Extract bit 383
    let (compression_bit, remainder) = felt_divmod(limb, 0x800000000000000000000000);
    
    // Extract bit 382
    let (infinity_bit, remainder) = felt_divmod(remainder, 0x400000000000000000000000);
    
    // Extract bit 381
    let (sign_bit, uncompressed_x_limb) = felt_divmod(remainder, 0x200000000000000000000000);

    // Construct the x coordinate of the point
    let x_point = UInt384(
        d0=compressed_g1.d0,
        d1=compressed_g1.d1,
        d2=compressed_g1.d2,
        d3=uncompressed_x_limb
    );

    return (CompressedG1Flags(compression_bit, infinity_bit, sign_bit), x_point);
}
from starkware.cairo.common.cairo_builtins import ModBuiltin
from starkware.cairo.common.uint256 import Uint256
from starkware.cairo.common.memcpy import memcpy
from starkware.cairo.common.alloc import alloc

from definitions import G1Point
from ec_ops import add_ec_points, is_on_curve_g1, sub_ec_points
from sha import HashUtils, SHA256
from cairo.src.types import SignerData

// // This file contains functions for aggregating public keys of signers in a BLS signature scheme.

// // Aggregate the signer's public keys by adding them all together.
// // Note: This function is not recommended for use as it is inefficient.
// func aggregate_signer_pubs{
//     range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
// }() -> (agg_pub: G1Point, n_signers: felt) {
//     alloc_locals;

//     // Allocate memory for signers and declare n_signers
//     let (signers: G1Point*) = alloc();
//     local n_signers: felt;

//     // Python hint to populate signers array and set n_signers
//     %{
//         from cairo.py.utils import generate_signers_array
//         signers = generate_signers_array(program_input["circuit_inputs"]["signers"])

//         for i, signer in enumerate(signers):
//             memory[ids.signers._reference_value + i * 8] = signer[0][0]
//             memory[ids.signers._reference_value + i * 8 + 1] = signer[0][1]
//             memory[ids.signers._reference_value + i * 8 + 2] = signer[0][2]
//             memory[ids.signers._reference_value + i * 8 + 3] = signer[0][3]
//             memory[ids.signers._reference_value + i * 8 + 4] = signer[1][0]
//             memory[ids.signers._reference_value + i * 8 + 5] = signer[1][1]
//             memory[ids.signers._reference_value + i * 8 + 6] = signer[1][2]
//             memory[ids.signers._reference_value + i * 8 + 7] = signer[1][3]

//         ids.n_signers = len(signers)
//     %}

//     // Call the recursive function to aggregate public keys
//     let (agg_pub) = aggregate_signer_pubs_inner(signers, n_signers);

//     return (agg_pub=agg_pub, n_signers=n_signers);
// }

// // Recursive helper function for aggregating signer public keys
// func aggregate_signer_pubs_inner{
//     range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
// }(signers: G1Point*, n_signers: felt) -> (res: G1Point) {
//     // Base case: if there's only one signer, verify it's on the curve and return it
//     if (n_signers == 1) {
//         let (on_curve) = is_on_curve_g1(1, [signers]);
//         assert on_curve = 1;
//         return ([signers],);
//     }

//     // Verify that the current signer's public key is on the curve
//     let (on_curve) = is_on_curve_g1(1, signers[0]);
//     assert on_curve = 1;

//     // Recursively aggregate the remaining public keys
//     let (res) = aggregate_signer_pubs_inner(signers + G1Point.SIZE, n_signers - 1);
//     // Add the current public key to the aggregated result
//     return add_ec_points(1, res, signers[0]);
// }

// // Aggregate the signer key by subtracting the non-signer keys from the committee key.
// // This approach is more efficient, using about 1/5 of the steps compared to add_ec_points.
// func fast_aggregate_signer_pubs{
//     range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
// }() -> (agg_pub: G1Point, n_non_signers: felt) {
//     alloc_locals;

//     // Allocate memory for non-signers and declare committee_pub and n_non_signers
//     let (non_signers: G1Point*) = alloc();
//     local committee_pub: G1Point;
//     local n_non_signers: felt;

//     // Python hint to populate non_signers array, set committee_pub, and n_non_signers
//     %{
//         from cairo.py.utils import generate_signers_array
//         non_signers = generate_signers_array(program_input["circuit_inputs"]["non_signers"])
//         write_g1(ids.committee_pub, program_input["circuit_inputs"]["sync_committee_agg_pub"])

//         for i, non_signer in enumerate(non_signers):
//             memory[ids.non_signers._reference_value + i * 8] = non_signer[0][0]
//             memory[ids.non_signers._reference_value + i * 8 + 1] = non_signer[0][1]
//             memory[ids.non_signers._reference_value + i * 8 + 2] = non_signer[0][2]
//             memory[ids.non_signers._reference_value + i * 8 + 3] = non_signer[0][3]
//             memory[ids.non_signers._reference_value + i * 8 + 4] = non_signer[1][0]
//             memory[ids.non_signers._reference_value + i * 8 + 5] = non_signer[1][1]
//             memory[ids.non_signers._reference_value + i * 8 + 6] = non_signer[1][2]
//             memory[ids.non_signers._reference_value + i * 8 + 7] = non_signer[1][3]

//         ids.n_non_signers = len(non_signers)
//     %}

//     // Call the recursive function to aggregate public keys
//     let (agg_pub) = fast_aggregate_signer_pubs_inner(committee_pub, non_signers, n_non_signers);

//     return (agg_pub=agg_pub, n_non_signers=n_non_signers);
// }

// // Recursive helper function for fast aggregation of signer public keys
// // This function starts from the committee aggregate key and subtracts the non-signer keys
// // to obtain the signer aggregate key
// func fast_aggregate_signer_pubs_inner{
//     range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
// }(agg_key: G1Point, non_signers: G1Point*, n_non_signers: felt) -> (res: G1Point) {
//     // Base case: if there are no non-signers, verify agg_key is on the curve and return it
//     if (n_non_signers == 0) {
//         let (on_curve) = is_on_curve_g1(1, agg_key);
//         assert on_curve = 1;
//         return (agg_key,);
//     }

//     // Verify that the current non-signer's public key is on the curve
//     let (on_curve) = is_on_curve_g1(1, non_signers[0]);
//     assert on_curve = 1;

//     // Recursively process the remaining non-signer keys
//     let (res) = fast_aggregate_signer_pubs_inner(
//         agg_key, non_signers + G1Point.SIZE, n_non_signers - 1
//     );
//     // Subtract the current non-signer's public key from the aggregated result
//     return sub_ec_points(1, res, non_signers[0]);  // try adding non signers and the subbing result
// }

func faster_fast_aggregate_signer_pubs{
    range_check_ptr,
    pow2_array: felt*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    sha256_ptr: felt*,
}(signer_data: SignerData) -> (committee_hash: Uint256, agg_pub: G1Point, n_non_signers: felt) {
    alloc_locals;

    let non_signers = signer_data.non_signers;
    let n_non_signers = signer_data.n_non_signers;
    let committee_pub = signer_data.committee_pub;

    // Python hint to populate non_signers array, set committee_pub, and n_non_signers
    // %{
    //     from cairo.py.utils import generate_signers_array
    //     non_signers = generate_signers_array(program_input["circuit_inputs"]["non_signers"])
    //     write_g1(ids.committee_pub, program_input["circuit_inputs"]["committee_pub"])

    //     for i, non_signer in enumerate(non_signers):
    //         memory[ids.non_signers._reference_value + i * 8] = non_signer[0][0]
    //         memory[ids.non_signers._reference_value + i * 8 + 1] = non_signer[0][1]
    //         memory[ids.non_signers._reference_value + i * 8 + 2] = non_signer[0][2]
    //         memory[ids.non_signers._reference_value + i * 8 + 3] = non_signer[0][3]
    //         memory[ids.non_signers._reference_value + i * 8 + 4] = non_signer[1][0]
    //         memory[ids.non_signers._reference_value + i * 8 + 5] = non_signer[1][1]
    //         memory[ids.non_signers._reference_value + i * 8 + 6] = non_signer[1][2]
    //         memory[ids.non_signers._reference_value + i * 8 + 7] = non_signer[1][3]

    //     ids.n_non_signers = len(non_signers)
    // %}

    // Call the recursive function to aggregate public keys
    if (n_non_signers != 0) {
        let (agg_non_signer_pub) = faster_fast_aggregate_signer_pubs_inner(
            non_signers, n_non_signers
        );
        let (signer_key) = sub_ec_points(1, committee_pub, agg_non_signer_pub);
        let committee_hash = commit_committee_key(point=committee_pub);
        return (committee_hash=committee_hash, agg_pub=signer_key, n_non_signers=n_non_signers);
    } else {
        let committee_hash = commit_committee_key(point=committee_pub);
        return (committee_hash=committee_hash, agg_pub=committee_pub, n_non_signers=n_non_signers);
    }
}

// Recursive helper function for fast aggregation of signer public keys
// In this version we add the non-signers (which is cheaper then subtracting)
// And then we subtract the result from the committee aggregate key. Saves avg 5k steps in normal epoch proof.
func faster_fast_aggregate_signer_pubs_inner{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(non_signers: G1Point*, n_non_signers: felt) -> (res: G1Point) {
    // Base case: if there are no non-signers, verify agg_key is on the curve and return it
    if (n_non_signers == 1) {
        let (on_curve) = is_on_curve_g1(1, non_signers[0]);
        assert on_curve = 1;
        return (non_signers[0],);
    }

    // Verify that the current non-signer's public key is on the curve
    let (on_curve) = is_on_curve_g1(1, non_signers[0]);
    assert on_curve = 1;

    // Recursively process the remaining non-signer keys
    let (res) = faster_fast_aggregate_signer_pubs_inner(
        non_signers + G1Point.SIZE, n_non_signers - 1
    );
    // Subtract the current non-signer's public key from the aggregated result
    return add_ec_points(1, res, non_signers[0]);  // try adding non signers and the subbing result
}

// This function generates the hash of an aggregate committee key.
// This hash is stored in the cairo1 state, and is used to check if the correct committee was used
func commit_committee_key{range_check_ptr, sha256_ptr: felt*, pow2_array: felt*}(
    point: G1Point
) -> Uint256 {
    alloc_locals;

    let (x_chunks) = HashUtils.chunk_uint384(point.x);
    let (y_chunks) = HashUtils.chunk_uint384(point.y);

    // Concatenate x and y chunks and compute the hash
    memcpy(dst=x_chunks + 12, src=y_chunks, len=12);
    let (committee_point_hash_chunks) = SHA256.hash_bytes(x_chunks, 96);
    let committee_point_hash = HashUtils.chunks_to_uint256(committee_point_hash_chunks);

    return committee_point_hash;
}

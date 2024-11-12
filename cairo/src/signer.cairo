from starkware.cairo.common.cairo_builtins import ModBuiltin
from definitions import G1Point
from ec_ops import add_ec_points, is_on_curve_g1, sub_ec_points
from starkware.cairo.common.alloc import alloc

// This file contains functions for aggregating public keys of signers in a BLS signature scheme.

// Aggregate the signer's public keys by adding them all together.
// Note: This function is not recommended for use as it is inefficient.
func aggregate_signer_pubs{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}() -> (agg_pub: G1Point, n_signers: felt) {
    alloc_locals;

    // Allocate memory for signers and declare n_signers
    let (signers: G1Point*) = alloc();
    local n_signers: felt;

    // Python hint to populate signers array and set n_signers
    %{
        from cairo.py.utils import generate_signers_array
        signers = generate_signers_array(program_input["signers"])

        for i, signer in enumerate(signers):
            memory[ids.signers._reference_value + i * 8] = signer[0][0]
            memory[ids.signers._reference_value + i * 8 + 1] = signer[0][1]
            memory[ids.signers._reference_value + i * 8 + 2] = signer[0][2]
            memory[ids.signers._reference_value + i * 8 + 3] = signer[0][3]
            memory[ids.signers._reference_value + i * 8 + 4] = signer[1][0]
            memory[ids.signers._reference_value + i * 8 + 5] = signer[1][1]
            memory[ids.signers._reference_value + i * 8 + 6] = signer[1][2]
            memory[ids.signers._reference_value + i * 8 + 7] = signer[1][3]

        ids.n_signers = len(signers)
    %}

    // Call the recursive function to aggregate public keys
    let (agg_pub) = aggregate_signer_pubs_inner(signers, n_signers);

    return (agg_pub=agg_pub, n_signers=n_signers);
}

// Recursive helper function for aggregating signer public keys
func aggregate_signer_pubs_inner {
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(signers: G1Point*, n_signers: felt) -> (res: G1Point) {
    // Base case: if there's only one signer, verify it's on the curve and return it
    if (n_signers == 1) {
        let (on_curve) = is_on_curve_g1(1, [signers]);
        assert on_curve = 1;
        return ([signers], );
    }

    // Verify that the current signer's public key is on the curve
    let (on_curve) = is_on_curve_g1(1, signers[0]);
    assert on_curve = 1;
    
    // Recursively aggregate the remaining public keys
    let (res) = aggregate_signer_pubs_inner(signers + G1Point.SIZE, n_signers - 1);
    // Add the current public key to the aggregated result
    return add_ec_points(1, res, signers[0]);
}

// Aggregate the signer key by subtracting the non-signer keys from the committee key.
// This approach is more efficient, using about 1/5 of the steps compared to add_ec_points.
func fast_aggregate_signer_pubs{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}() -> (agg_pub: G1Point, n_non_signers: felt) {
    alloc_locals;

    // Allocate memory for non-signers and declare committee_pub and n_non_signers
    let (non_signers: G1Point*) = alloc();
    local committee_pub: G1Point;
    local n_non_signers: felt;

    // Python hint to populate non_signers array, set committee_pub, and n_non_signers
    %{
        from cairo.py.utils import generate_signers_array
        non_signers = generate_signers_array(program_input["aggregates"]["nonSigners"])
        write_g1(ids.committee_pub, program_input["aggregates"]["committee"])

        for i, non_signer in enumerate(non_signers):
            memory[ids.non_signers._reference_value + i * 8] = non_signer[0][0]
            memory[ids.non_signers._reference_value + i * 8 + 1] = non_signer[0][1]
            memory[ids.non_signers._reference_value + i * 8 + 2] = non_signer[0][2]
            memory[ids.non_signers._reference_value + i * 8 + 3] = non_signer[0][3]
            memory[ids.non_signers._reference_value + i * 8 + 4] = non_signer[1][0]
            memory[ids.non_signers._reference_value + i * 8 + 5] = non_signer[1][1]
            memory[ids.non_signers._reference_value + i * 8 + 6] = non_signer[1][2]
            memory[ids.non_signers._reference_value + i * 8 + 7] = non_signer[1][3]

        ids.n_non_signers = len(non_signers)
    %}

    // Call the recursive function to aggregate public keys
    let (agg_pub) = fast_aggregate_signer_pubs_inner(committee_pub, non_signers, n_non_signers);

    return (agg_pub=agg_pub, n_non_signers=n_non_signers);
}

// Recursive helper function for fast aggregation of signer public keys
// This function starts from the committee aggregate key and subtracts the non-signer keys
// to obtain the signer aggregate key
func fast_aggregate_signer_pubs_inner{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(agg_key: G1Point, non_signers: G1Point*, n_non_signers: felt) -> (res: G1Point) {
     // Base case: if there are no non-signers, verify agg_key is on the curve and return it
     if (n_non_signers == 0) {
        let (on_curve) = is_on_curve_g1(1, agg_key);
        assert on_curve = 1;
        return (agg_key, );
    }

    // Verify that the current non-signer's public key is on the curve
    let (on_curve) = is_on_curve_g1(1, non_signers[0]);
    assert on_curve = 1;
    
    // Recursively process the remaining non-signer keys
    let (res) = fast_aggregate_signer_pubs_inner(agg_key, non_signers + G1Point.SIZE, n_non_signers - 1);
    // Subtract the current non-signer's public key from the aggregated result
    return sub_ec_points(1, res, non_signers[0]); // try adding non signers and the subbing result
}
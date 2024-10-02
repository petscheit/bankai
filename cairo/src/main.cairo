%builtins range_check poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from cairo.src.constants import g1_negative

from definitions import bn, bls, UInt384, one_E12D, N_LIMBS, BASE, E12D, G1Point, G2Point, G1G2Pair
from ec_ops import add_ec_points, is_on_curve_g1
from pairing import multi_pairing

func main{
    range_check_ptr,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;

    let (agg_pub, n_signers) = aggregate_signer_pubs();

    local msg_point: G2Point;
    local sig_point: G2Point;
    %{
        from cairo.py.utils import write_g2, write_g1g2

        write_g2(ids.sig_point, program_input["sig"])
        write_g2(ids.msg_point, program_input["msg"])
    %}
    verify_signature(agg_pub, msg_point, sig_point);

    return ();
}

func verify_signature{
    range_check_ptr, poseidon_ptr: PoseidonBuiltin*, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(agg_pub: G1Point, msg_point: G2Point, sig_point: G2Point) {
    
    let neg_g1: G1Point = g1_negative();
    let g1_sig_pair: G1G2Pair = G1G2Pair(P=neg_g1, Q=sig_point);
    let pk_msg_pair: G1G2Pair = G1G2Pair(P=agg_pub, Q=msg_point);

    let (inputs: G1G2Pair*) = alloc();  
    assert inputs[0] = g1_sig_pair;
    assert inputs[1] = pk_msg_pair;

    // We check the pairs are on the curve in the pairing function
    let (res) = multi_pairing(inputs, 2, 1);
    let (one) = one_E12D();
    assert res = one;
    return ();

}


// Per aggregation, this costs around 890 steps. Currently we add all the signer keys together, which is quite wasteful.
// In the future, we should subtract the non-signers from the aggregate key of the current committee
func aggregate_signer_pubs{
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}() -> (agg_pub: G1Point, n_signers: felt) {
    alloc_locals;

    let (signers: G1Point*) = alloc();
    local n_signers: felt;
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

    let (agg_pub) = aggregate_signer_pubs_inner(signers, n_signers);

    return (agg_pub=agg_pub, n_signers=n_signers);
}

func aggregate_signer_pubs_inner {
    range_check_ptr, range_check96_ptr: felt*, add_mod_ptr: ModBuiltin*, mul_mod_ptr: ModBuiltin*
}(signers: G1Point*, n_signers: felt) -> (res: G1Point) {
    if (n_signers == 1) {
        let (on_curve) = is_on_curve_g1(1, [signers]);
        assert on_curve = 1;
        return ([signers], );
    }

    let (on_curve) = is_on_curve_g1(1, signers[0]);
    assert on_curve = 1;
    
    let (res) = aggregate_signer_pubs_inner(signers + G1Point.SIZE, n_signers - 1);
    return add_ec_points(1, res, signers[0]);

}


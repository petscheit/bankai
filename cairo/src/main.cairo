%builtins output range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from definitions import bn, bls, UInt384, one_E12D, N_LIMBS, BASE, E12D, G1Point, G2Point, G1G2Pair
from ec_ops import add_ec_points, is_on_curve_g1
from pairing import multi_pairing
from cairo.src.ssz import SSZ
from starkware.cairo.common.uint256 import Uint256
from cairo.src.constants import g1_negative
from cairo.src.domain import Domain

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

    // let (agg_pub, n_signers) = aggregate_signer_pubs();
    local agg_key: G1Point;
    local msg_point: G2Point;
    local sig_point: G2Point;
    local slot: felt;
    %{
        from cairo.py.utils import write_g2, write_g1g2, write_g1
        write_g1(ids.agg_key, program_input["proofPoints"]["publicKey"])
        write_g2(ids.sig_point, program_input["proofPoints"]["signature"])
        write_g2(ids.msg_point, program_input["proofPoints"]["msg"])
        ids.slot = int(program_input["header"]["slot"], 10)
    %}
    %{ print("Running Verification for Slot: ", ids.slot) %}

    let (header_root) = hash_header();
    %{ print("HeaderRoot: ", hex(ids.header_root.high * 2**128 + ids.header_root.low)) %}

    let signing_root = Domain.compute_signing_root(header_root, slot);
    %{ print("SigningRoot: ", hex(ids.signing_root.high * 2**128 + ids.signing_root.low)) %}
    let (_agg_key, n_signers) = aggregate_signer_pubs();
    %{ print("N_Signers: ", ids.n_signers) %}

    with_attr error_message("NOT_ENOUGH_SIGNERS") {
        // this ensures more then 80% of the committee signed the block
        assert [range_check_ptr] = n_signers - 410;
        tempvar range_check_ptr = range_check_ptr + 1;
    }

    verify_signature(agg_key, msg_point, sig_point);

    assert [output_ptr] = header_root.low;
    assert [output_ptr + 1] = header_root.high;
    let output_ptr = output_ptr + 2;

    return ();
}

func hash_header{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
}() -> (header_root: Uint256) {
    alloc_locals;

    local slot: Uint256;
    local proposer_index: Uint256;
    local parent_root: Uint256;
    local state_root: Uint256;
    local body_root: Uint256;
    %{  
        from cairo.py.utils import split_uint256
        ids.slot.low = int(program_input["header"]["slot"], 10)
        ids.slot.high = 0

        ids.proposer_index.low = int(program_input["header"]["proposer_index"], 10)
        ids.proposer_index.high = 0

        parent_root = split_uint256(int(program_input["header"]["parent_root"], 16))
        ids.parent_root.low, ids.parent_root.high = parent_root

        state_root = split_uint256(int(program_input["header"]["state_root"], 16))
        ids.state_root.low, ids.state_root.high = state_root

        body_root = split_uint256(int(program_input["header"]["body_root"], 16))
        ids.body_root.low, ids.body_root.high = body_root
    %}

    let header_root = SSZ.hash_header_root(slot, proposer_index, parent_root, state_root, body_root);

    return (header_root=header_root);
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


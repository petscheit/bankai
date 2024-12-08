%builtins output range_check bitwise poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.uint256 import Uint256
from definitions import bn, bls, UInt384, one_E12D, N_LIMBS, BASE, E12D, G1Point, G2Point, G1G2Pair
from bls12_381.multi_pairing_2 import multi_pairing_2P
from hash_to_curve import hash_to_curve
from cairo.src.ssz import SSZ
from cairo.src.constants import g1_negative
from cairo.src.domain import Domain
from cairo.src.signer import fast_aggregate_signer_pubs, aggregate_signer_pubs, faster_fast_aggregate_signer_pubs
from cairo.src.utils import pow2alloc128
from sha import SHA256

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

    let (pow2_array) = pow2alloc128();
    let (sha256_ptr, sha256_ptr_start) = SHA256.init();

    local sig_point: G2Point;
    local slot: felt;
    %{
        from cairo.py.utils import write_g2, write_g1g2, write_g1, print_g2
        write_g2(ids.sig_point, program_input["signature_point"])
        ids.slot = int(program_input["header"]["slot"], 16)
    %}
    %{ print("Running Verification for Slot: ", ids.slot) %}

    with pow2_array, sha256_ptr {
        let (header_root) = hash_header();
    }
    %{ print("HeaderRoot: ", hex(ids.header_root.high * 2**128 + ids.header_root.low)) %}

    with pow2_array, sha256_ptr {
        let signing_root = Domain.compute_signing_root(header_root, slot);
    }
    %{ print("SigningRoot: ", hex(ids.signing_root.high * 2**128 + ids.signing_root.low)) %}

    with pow2_array, sha256_ptr {
        let (msg_point) = hash_to_curve(1, signing_root);
    }

    %{ print_g2("MsgPoint", ids.msg_point) %}

    with sha256_ptr, pow2_array {
        let (committee_hash, agg_key, n_non_signers) = faster_fast_aggregate_signer_pubs();
    }
    let n_signers = 512 - n_non_signers;

    with_attr error_message("NOT_ENOUGH_SIGNERS") {
        assert [range_check_ptr] = n_signers - 410;
        tempvar range_check_ptr = range_check_ptr + 1;
    }

    SHA256.finalize(sha256_start_ptr=sha256_ptr_start, sha256_end_ptr=sha256_ptr);
    verify_signature(agg_key, msg_point, sig_point);

    assert [output_ptr] = header_root.low;
    assert [output_ptr + 1] = header_root.high;
    assert [output_ptr + 2] = committee_hash.low;
    assert [output_ptr + 3] = committee_hash.high;
    assert [output_ptr + 4] = n_signers;
    let output_ptr = output_ptr + 5;

    return ();
}

func hash_header{
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    pow2_array: felt*,
    sha256_ptr: felt*
}() -> (header_root: Uint256) {
    alloc_locals;

    local slot: Uint256;
    local proposer_index: Uint256;
    local parent_root: Uint256;
    local state_root: Uint256;
    local body_root: Uint256;
    %{  
        from cairo.py.utils import split_uint256
        ids.slot.low = int(program_input["header"]["slot"], 16)
        ids.slot.high = 0

        ids.proposer_index.low = int(program_input["header"]["proposer_index"], 16)
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
    let (res) = multi_pairing_2P(inputs);
    let (one) = one_E12D();
    assert res = one;
    return ();

}
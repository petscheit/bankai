from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin, BitwiseBuiltin
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.uint256 import Uint256
from definitions import bn, bls, UInt384, one_E12D, N_LIMBS, BASE, E12D, G1Point, G2Point, G1G2Pair
from bls12_381.multi_pairing_2 import multi_pairing_2P
from hash_to_curve import hash_to_curve
from cairo.src.ssz import SSZ, MerkleTree, MerkleUtils
from cairo.src.constants import g1_negative
from cairo.src.domain import Domain
from cairo.src.signer import (
    faster_fast_aggregate_signer_pubs,
)
from cairo.src.utils import pow2alloc128
from sha import SHA256
from debug import print_string, print_felt_hex, print_felt
from cairo.src.types import SignerData, ExecutionHeaderProof, BeaconHeader, EpochUpdate

func run_epoch_update{
    output_ptr: felt*,
    range_check_ptr,
    bitwise_ptr: BitwiseBuiltin*,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
    pow2_array: felt*,
    sha256_ptr: felt*,
}(epoch_update: EpochUpdate) {
    alloc_locals;

    print_string('run epoch update');

    // 1. Hash beacon header
    let (header_root, body_root, state_root) = hash_header(epoch_update.header);

    // 2. Compute signing root (this is what validators sign)
    let signing_root = Domain.compute_signing_root(header_root, epoch_update.header.slot.low);

    // 3. Hash to curve to get message point
    let (msg_point) = hash_to_curve(1, signing_root);

    // 4. Aggregate signer to get aggregate key that was used to sign the message
    let (committee_hash, agg_key, n_non_signers) = faster_fast_aggregate_signer_pubs(epoch_update.signer_data);
    let n_signers = 512 - n_non_signers;

    // 5. Verify signature
    verify_signature(agg_key, msg_point, epoch_update.sig_point);

    // 6. Hash execution payload root (SSZ encoded execution payload) which is stored in the beacon state
    let (execution_root, execution_hash, execution_height) = SSZ.hash_execution_payload_header_root(epoch_update.execution_header_proof.payload_fields);

    // 7. Verify ssz inclusion proof
    let root_felts = MerkleUtils.chunk_uint256(execution_root);
    let computed_body_root = MerkleTree.hash_merkle_path(
        path=epoch_update.execution_header_proof.path, path_len=4, leaf=root_felts, index=9
    );

    // 8. Assert that the computed body root matches the body root of the verified header
    assert computed_body_root.low = body_root.low;
    assert computed_body_root.high = body_root.high;

    // %{ verify_epoch_update_outputs() %}

    assert [output_ptr] = header_root.low;
    assert [output_ptr + 1] = header_root.high;
    assert [output_ptr + 2] = state_root.low;
    assert [output_ptr + 3] = state_root.high;
    assert [output_ptr + 4] = epoch_update.header.slot.low;
    assert [output_ptr + 5] = committee_hash.low;
    assert [output_ptr + 6] = committee_hash.high;
    assert [output_ptr + 7] = n_signers;
    assert [output_ptr + 8] = execution_hash.low;
    assert [output_ptr + 9] = execution_hash.high;
    assert [output_ptr + 10] = execution_height;
    let output_ptr = output_ptr + 11;

    return ();
}

func hash_header{
    range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
}(header: BeaconHeader) -> (header_root: Uint256, body_root: Uint256, state_root: Uint256) {
    alloc_locals;

    let header_root = SSZ.hash_header_root(
        header.slot, header.proposer_index, header.parent_root, header.state_root, header.body_root
    );

    return (header_root=header_root, body_root=header.body_root, state_root=header.state_root);
}

func verify_signature{
    range_check_ptr,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
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

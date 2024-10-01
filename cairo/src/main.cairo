%builtins range_check poseidon range_check96 add_mod mul_mod

from starkware.cairo.common.cairo_builtins import PoseidonBuiltin, ModBuiltin
from starkware.cairo.common.registers import get_fp_and_pc
from starkware.cairo.common.alloc import alloc
from cairo.src.constants import g1_negative

from definitions import bn, bls, UInt384, one_E12D, N_LIMBS, BASE, E12D, G1Point, G2Point, G1G2Pair
from pairing import multi_pairing
from modulo_circuit import ExtensionFieldModuloCircuit

func main{
    range_check_ptr,
    poseidon_ptr: PoseidonBuiltin*,
    range_check96_ptr: felt*,
    add_mod_ptr: ModBuiltin*,
    mul_mod_ptr: ModBuiltin*,
}() {
    alloc_locals;


    local pk_msg_pair: G1G2Pair;
    local sig_point: G2Point;
    %{
        from cairo.py.utils import write_g2, write_g1g2

        write_g2(ids.sig_point, program_input["sig"])
        write_g1g2(ids.pk_msg_pair, program_input["pub"], program_input["msg"])
    %}
    let neg_g1: G1Point = g1_negative();
    let g1_sig_pair: G1G2Pair = G1G2Pair(P=neg_g1, Q=sig_point);

    let (inputs: G1G2Pair*) = alloc();  
    assert inputs[0] = g1_sig_pair;
    assert inputs[1] = pk_msg_pair;

    let (res) = multi_pairing(inputs, 2, 1);
    let (one) = one_E12D();
    assert res = one;

    return ();
}
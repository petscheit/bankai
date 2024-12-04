from py_ecc.optimized_bls12_381 import (
    hash_to_G2,
    hash_to_G1,
)

def g1_hash_to_curve(low: int, high: int) -> tuple[int, int]:
    concat = low + (high << 128)
    return hash_to_G1(concat)
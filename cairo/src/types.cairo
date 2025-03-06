from definitions import G1Point
from starkware.cairo.common.uint256 import Uint256

struct SignerData {
    committee_pub: G1Point,
    non_signers: G1Point*,
    n_non_signers: felt,
}
struct ExecutionHeaderProof {
    root: Uint256,
    path: felt**,
    leaf: Uint256,
    index: felt,
    payload_fields: Uint256*,
}
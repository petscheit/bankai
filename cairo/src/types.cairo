from definitions import G1Point

struct SignerData {
    committee_pub: G1Point,
    non_signers: G1Point*,
    n_non_signers: felt,
}
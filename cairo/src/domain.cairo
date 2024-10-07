
from starkware.cairo.common.cairo_builtins import KeccakBuiltin, BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from cairo.src.ssz import SSZ
from cairo.src.utils import felt_divmod

namespace Domain {
    // 0x07000000 is the real sync committee domain, but we can asign the shifted value already
    const DOMAIN_SYNC_COMMITTEE = 0x07000000000000000000000000000000;

    func compute_domain{
        range_check_ptr,
    }(slot: felt) -> Uint256 {
        let fork_root = ForkSepolia.get_fork_root(slot);
        
        // We now need to right right-shift the fork root 4 bytes, and prepend the domain
        let (q_high, r_high) = felt_divmod(fork_root.high, 0x100000000);
        let (q_low, _r_low) = felt_divmod(fork_root.low, 0x100000000);

        let high = DOMAIN_SYNC_COMMITTEE + q_high;
        let low = r_high * 0x1000000000000000000000000 + q_low;

        return (Uint256(low=low, high=high));
    }

    func compute_signing_root{
        range_check_ptr,
    }(message:Uint256, slot: felt) -> Uint256 {
        let domain = compute_domain(slot);
        let root = SSZ.hash_pair_container(message, domain);
        return root;
    }
}

// ToDo: these should be precomputed and hardcoded
namespace ForkSepolia {
    func genesis_validator_root() -> Uint256 {
        return (Uint256(low=0xcf3f9209c00e4efbaaddac09ed9b8078, high=0xd8ea171f3c94aea21ebc42a1ed61052a));
    }

    const GENESIS_ACTIVATION_SLOT = 0; // 0 * SLOTS_PER_EPOCH;
    const ALTAIR_ACTIVATION_SLOT = 1600; // 50 * SLOTS_PER_EPOCH;
    const BELLATRIX_ACTIVATION_SLOT = 3200; // 100 * SLOTS_PER_EPOCH;
    const CAPPELLA_ACTIVATION_SLOT = 1818624; //56832 * SLOTS_PER_EPOCH;
    const DENEB_ACTIVATION_SLOT = 4243456; //132608 * SLOTS_PER_EPOCH;

    func get_fork_version{
        range_check_ptr,
    }(slot: felt) -> felt {
        alloc_locals;
        
        local fork: felt;
        %{
            if ids.slot < ids.ALTAIR_ACTIVATION_SLOT:
                ids.fork = 0
            elif ids.slot < ids.BELLATRIX_ACTIVATION_SLOT:
                ids.fork = 1
            elif ids.slot < ids.CAPPELLA_ACTIVATION_SLOT:
                ids.fork = 2
            elif ids.slot < ids.DENEB_ACTIVATION_SLOT:
                ids.fork = 3
            else:
                ids.fork = 4
        %}

        if (fork == 0) {
            assert [range_check_ptr] = ALTAIR_ACTIVATION_SLOT - slot;
            tempvar range_check_ptr = range_check_ptr + 1;
            return 0x90000069000000000000000000000000;
        } 
        
        if (fork == 1) {
            assert [range_check_ptr] = BELLATRIX_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - ALTAIR_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return 0x90000070000000000000000000000000;
        }
        
        if (fork == 2) {
            assert [range_check_ptr] = CAPPELLA_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - BELLATRIX_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return 0x90000071000000000000000000000000;
        }

        if (fork == 3) {
            assert [range_check_ptr] = DENEB_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - CAPPELLA_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return 0x90000072000000000000000000000000;
        }
        return 0x90000073000000000000000000000000;
    }

    func get_fork_root{
        range_check_ptr,
    }(slot: felt) -> Uint256 {
        let fork = get_fork_version(slot);
        let genesis_validator_root = ForkSepolia.genesis_validator_root();
        let root = SSZ.hash_pair_container(Uint256(low=0, high=fork), genesis_validator_root);
        return root;
    }
}

// func main{
//     range_check_ptr,
//     bitwise_ptr: BitwiseBuiltin*,
//     keccak_ptr: KeccakBuiltin*
// }() {
//     let slot = 5800064;
//     let message = Uint256(low=0x0301f5a1c008e421763bf6a8216cad26, high=0xc9e2eb8ea4c28d03eefbf5d73bdbbabb);
//     let signing_root = Domain.compute_signing_root(message, slot);
//     assert signing_root.low = 0x63369ab49953ce2de3fc0021de97fabc;
//     assert signing_root.high = 0x6e23ded16bf8cd3f565d7b7503ee557f;
//     return ();
// }
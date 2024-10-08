
from starkware.cairo.common.cairo_builtins import KeccakBuiltin, BitwiseBuiltin
from starkware.cairo.common.uint256 import Uint256
from cairo.src.ssz import SSZ
from cairo.src.utils import felt_divmod

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

    // This function uses precomuted domain values
    func get_domain{
        range_check_ptr,
    }(slot: felt) -> Uint256 {
        alloc_locals;

        local fork: felt;
        %{
            if ids.slot < ids.ForkSepolia.ALTAIR_ACTIVATION_SLOT:
                ids.fork = 0
            elif ids.slot < ids.ForkSepolia.BELLATRIX_ACTIVATION_SLOT:
                ids.fork = 1
            elif ids.slot < ids.ForkSepolia.CAPPELLA_ACTIVATION_SLOT:
                ids.fork = 2
            elif ids.slot < ids.ForkSepolia.DENEB_ACTIVATION_SLOT:
                ids.fork = 3
            else:
                ids.fork = 4
        %}

        if (fork == 0) {
            assert [range_check_ptr] = ForkSepolia.ALTAIR_ACTIVATION_SLOT - slot;
            tempvar range_check_ptr = range_check_ptr + 1;
            return (Uint256(low=0x5f699a49ccd9b3fd666c35d4ae5f79e, high=0x7000000a8fee8ee9978418b64f1140b));
        } 
        
        if (fork == 1) {
            assert [range_check_ptr] = ForkSepolia.BELLATRIX_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - ForkSepolia.ALTAIR_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return (Uint256(low=0xd45d10c47a506110d536465a1b29c3fe, high=0x700000060725ca1ffa72cacd1eedcf7));
        }
        
        if (fork == 2) {
            assert [range_check_ptr] = ForkSepolia.CAPPELLA_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - ForkSepolia.BELLATRIX_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return (Uint256(low=0x10839ea6dcaaaa6372e95478610d7e08, high=0x700000036fa50131482fe2af396daf2));
        }

        if (fork == 3) {
            assert [range_check_ptr] = ForkSepolia.DENEB_ACTIVATION_SLOT - slot;
            assert [range_check_ptr + 1] = slot - ForkSepolia.CAPPELLA_ACTIVATION_SLOT;
            tempvar range_check_ptr = range_check_ptr + 2;
            return (Uint256(low=0x60f0a2ed78c1a85f0654941a0d19d0fa, high=0x700000047eb72b3be36f08feffcaba7));
        }
        return (Uint256(low=0x55fcf34b7e308f8fbca8e663bf565808, high=0x7000000d31f6191ca65c836e170318c));
    }

    func compute_signing_root{
        range_check_ptr,
    }(message:Uint256, slot: felt) -> Uint256 {
        let domain = get_domain(slot);
        let root = SSZ.hash_pair_container(message, domain);
        return root;
    }
}

//  func precompute_domains{
//     range_check_ptr,
// }() {
//     let genesis_domain = Domain.compute_domain(ForkSepolia.GENESIS_ACTIVATION_SLOT);
//     %{ print(f"genesis_domain: Uint256(low={hex(ids.genesis_domain.low)}, high={hex(ids.genesis_domain.high)})") %}
//     let altair_domain = Domain.compute_domain(ForkSepolia.ALTAIR_ACTIVATION_SLOT);
//     %{ print(f"altair_domain: Uint256(low={hex(ids.altair_domain.low)}, high={hex(ids.altair_domain.high)})") %}
//     let bellatrix_domain = Domain.compute_domain(ForkSepolia.BELLATRIX_ACTIVATION_SLOT);
//     %{ print(f"bellatrix_domain: Uint256(low={hex(ids.bellatrix_domain.low)}, high={hex(ids.bellatrix_domain.high)})") %}
//     let capella_domain = Domain.compute_domain(ForkSepolia.CAPPELLA_ACTIVATION_SLOT);
//     %{ print(f"capella_domain: Uint256(low={hex(ids.capella_domain.low)}, high={hex(ids.capella_domain.high)})") %}
//     let deneb_domain = Domain.compute_domain(ForkSepolia.DENEB_ACTIVATION_SLOT);
//     %{ print(f"deneb_domain: Uint256(low={hex(ids.deneb_domain.low)}, high={hex(ids.deneb_domain.high)})") %}
//     return ();
// }

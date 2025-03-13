from starkware.cairo.common.cairo_builtins import KeccakBuiltin, BitwiseBuiltin
from starkware.cairo.common.registers import get_label_location
from starkware.cairo.common.uint256 import Uint256
from cairo.src.ssz import SSZ
from cairo.src.utils import felt_divmod
from debug import print_string, print_felt_hex

namespace Network {
    // Network IDs
    const MAINNET = 0;
    const SEPOLIA = 1;

    // Fork IDs
    const GENESIS = 0;
    const ALTAIR = 1;
    const BELLATRIX = 2;
    const CAPELLA = 3;
    const DENEB = 4;
    const ELECTRA = 5;

    func get_genesis_validator_root(network_id: felt) -> Uint256 {
        if (network_id == Network.MAINNET) {
            return (Uint256(low=0x54bfe9f06bf33ff6cf5ad27f511bfe95, high=0x4b363db94e286120d76eb905340fdd4e));
        } else {
            return (Uint256(low=0xcf3f9209c00e4efbaaddac09ed9b8078, high=0xd8ea171f3c94aea21ebc42a1ed61052a));
        }
    }

    func get_fork_version{range_check_ptr}(network_id: felt, slot: felt) -> felt {
        alloc_locals;

        local fork: felt;
        let (fork_schedule) = get_fork_schedule(); // required for hint, no great way to do this
        %{ check_fork_version() %}

        return fork;
    }

    func get_fork_root{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*}(network_id: felt, slot: felt) -> Uint256 {
        alloc_locals;
        let fork_id = get_fork_id(network_id, slot);
        let genesis_validator_root = get_genesis_validator_root(network_id);

        let root = SSZ.hash_pair_container(Uint256(low=0, high=fork_id), genesis_validator_root);
        return root;
    }

    func get_fork_id{range_check_ptr}(network_id: felt, slot: felt) -> felt {
        alloc_locals;

        let fork = get_fork_version(network_id, slot);

        if (fork == Network.GENESIS) {
            let (fork_id, _) = get_fork_data(network_id, Network.GENESIS);
            let (_, altair_activation_slot) = get_fork_data(network_id, Network.ALTAIR);
            assert [range_check_ptr] = altair_activation_slot - slot;
            tempvar range_check_ptr = range_check_ptr + 1;
            return fork_id;
        }

        if (fork == Network.ALTAIR) {
            let (_, altair_activation_slot) = get_fork_data(network_id, Network.ALTAIR);
            let (_, bellatrix_activation_slot) = get_fork_data(network_id, Network.BELLATRIX);
            assert [range_check_ptr] = bellatrix_activation_slot - slot;
            assert [range_check_ptr + 1] = slot - altair_activation_slot;
            tempvar range_check_ptr = range_check_ptr + 2;
            return fork;
        }

        if (fork == Network.BELLATRIX) {
            let (fork_id, bellatrix_activation_slot) = get_fork_data(network_id, Network.BELLATRIX);
            let (_, capella_activation_slot) = get_fork_data(network_id, Network.CAPELLA);
            assert [range_check_ptr] = capella_activation_slot - slot;
            assert [range_check_ptr + 1] = slot - bellatrix_activation_slot;
            tempvar range_check_ptr = range_check_ptr + 2;
            return fork_id;
        }

        if (fork == Network.CAPELLA) {
            let (fork_id, capella_activation_slot) = get_fork_data(network_id, Network.CAPELLA);
            let (_, deneb_activation_slot) = get_fork_data(network_id, Network.DENEB);
            assert [range_check_ptr] = deneb_activation_slot - slot;
            assert [range_check_ptr + 1] = slot - capella_activation_slot;
            tempvar range_check_ptr = range_check_ptr + 2;
            return fork_id;
        }

        if (fork == Network.DENEB) {
            let (fork_id, deneb_activation_slot) = get_fork_data(network_id, Network.DENEB);
            let (_, electra_activation_slot) = get_fork_data(network_id, Network.ELECTRA);
            assert [range_check_ptr] = electra_activation_slot - slot;
            assert [range_check_ptr + 1] = slot - deneb_activation_slot;
            tempvar range_check_ptr = range_check_ptr + 2;
            return fork_id;
        }

        if (fork == Network.ELECTRA) {
            let (fork_id, electra_activation_slot) = get_fork_data(network_id, Network.ELECTRA);
            assert [range_check_ptr] = electra_activation_slot - slot;
            tempvar range_check_ptr = range_check_ptr + 1;
            return fork_id;
        }

        assert 1 = 0;
        return 0xFFFFFFFFFFFFFFFF;
    }
    
    // Data structure for fork versions and activation slots
    func get_fork_data{range_check_ptr}(network_id: felt, fork_id: felt) -> (version: felt, slot: felt) {
        alloc_locals;
        
        assert [range_check_ptr] = 2 - network_id;  // Check network_id is valid (0 or 1)
        assert [range_check_ptr + 1] = 5 - fork_id;  // Check fork_id is valid (0-5)
        tempvar range_check_ptr = range_check_ptr + 2;
        
        let (fork_schedule) = get_fork_schedule();
        
        // Each network has 12 values (6 forks × 2 values per fork)
        // For each fork: [version, slot]
        local version = [fork_schedule + (fork_id * 2) + (12 * network_id)];
        local slot = [fork_schedule + (fork_id * 2) + 1 + (12 * network_id)];
        
        return (version, slot);
    }

    func get_fork_schedule() -> (felt*) {
        return get_label_location(fork_schedule);

        fork_schedule:
        // MAINNET fork data (version, slot)
        // GENESIS
        dw 0x00000000000000000000000000000000; // GENESIS_FORK_VERSION
        dw 0;                                  // GENESIS_ACTIVATION_SLOT
        // ALTAIR
        dw 0x01000000000000000000000000000000; // ALTAIR_FORK_VERSION
        dw 2375680;                            // ALTAIR_ACTIVATION_SLOT (74240 * 32)
        // BELLATRIX
        dw 0x02000000000000000000000000000000; // BELLATRIX_FORK_VERSION
        dw 4636672;                            // BELLATRIX_ACTIVATION_SLOT (144896 * 32)
        // CAPELLA
        dw 0x03000000000000000000000000000000; // CAPELLA_FORK_VERSION
        dw 6209536;                            // CAPELLA_ACTIVATION_SLOT (194048 * 32)
        // DENEB
        dw 0x04000000000000000000000000000000; // DENEB_FORK_VERSION
        dw 8626176;                            // DENEB_ACTIVATION_SLOT (269568 * 32)
        // ELECTRA
        dw 0x05000000000000000000000000000000; // ELECTRA_FORK_VERSION
        dw 0xFFFFFFFFFFFFFFFF;                 // ELECTRA_ACTIVATION_SLOT (Infinity)
        
        // SEPOLIA fork data (version, slot)
        // GENESIS
        dw 0x90000069000000000000000000000000; // GENESIS_FORK_VERSION
        dw 0;                                  // GENESIS_ACTIVATION_SLOT
        // ALTAIR
        dw 0x90000070000000000000000000000000; // ALTAIR_FORK_VERSION
        dw 1600;                               // ALTAIR_ACTIVATION_SLOT (50 * 32)
        // BELLATRIX
        dw 0x90000071000000000000000000000000; // BELLATRIX_FORK_VERSION
        dw 3200;                               // BELLATRIX_ACTIVATION_SLOT (100 * 32)
        // CAPELLA
        dw 0x90000072000000000000000000000000; // CAPELLA_FORK_VERSION
        dw 1818624;                            // CAPELLA_ACTIVATION_SLOT (56832 * 32)
        // DENEB
        dw 0x90000073000000000000000000000000; // DENEB_FORK_VERSION
        dw 4243456;                            // DENEB_ACTIVATION_SLOT (132608 * 32)
        // ELECTRA
        dw 0x90000074000000000000000000000000; // ELECTRA_FORK_VERSION
        dw 7118848;                            // ELECTRA_ACTIVATION_SLOT (222464 * 32)
    }
}

namespace Domain {
    const DOMAIN_SYNC_COMMITTEE = 0x07000000000000000000000000000000;

    func compute_signing_root{
        range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*
    }(network_id: felt, message: Uint256, slot: felt) -> Uint256 {
        let domain = get_domain(network_id, slot);
        let root = SSZ.hash_pair_container(message, domain);
        return root;
    }

    func get_domain{range_check_ptr}(network_id: felt, slot: felt) -> Uint256 {
        alloc_locals;

        let fork = Network.get_fork_version(network_id, slot);

        if (network_id == Network.MAINNET) {
            let (data_address) = get_label_location(domain_data_mainnet);
            let low = [data_address + fork * 2];
            let high = [data_address + fork * 2 + 1];
            return (Uint256(low=low, high=high));
        } else {
            let (data_address) = get_label_location(domain_data_sepolia);
            let low = [data_address + fork * 2];
            let high = [data_address + fork * 2 + 1];
            return (Uint256(low=low, high=high));
        }

        // MAINNET dummy precomputed domain values.
        domain_data_mainnet:
        dw 0x2350947421a3e4a979779642cfdb0f66; // Genesis low
        dw 0x7000000b5303f2ad2010d699a76c8e6; // Genesis high

        dw 0x31016c31b4da651f362045e02b4447f0; // Altair low
        dw 0x7000000c3442b13b42f0f3c37034be9; // Altair high

        dw 0x40848881a8d4f0af0be83417a85c0f45; // Bellatrix low
        dw 0x70000004a26c58b08add8089b75caa5; // Bellatrix high

        dw 0x69bf583a7f9e0af049305b62de676640; // Capella low
        dw 0x7000000bba4da96354c9f25476cf1bc; // Capella high

        dw 0x883b712607f952d5198d0f5677564636; // Deneb low
        dw 0x70000006a95a1a967855d676d48be69; // Deneb high

        dw 0x883b712607f952d5198d0f5677564636; // Electra low
        dw 0x70000006a95a1a967855d676d48be69; // Electra high

        domain_data_sepolia:
        dw 0x5f699a49ccd9b3fd666c35d4ae5f79e; // Genesis low
        dw 0x7000000a8fee8ee9978418b64f1140b; // Genesis high

        dw 0x32399a96f89d5ce37f1b875852afd540; // Altair low
        dw 0x70000002944546c0d50cbdfd9448dfc; // Altair high

        dw 0x10839ea6dcaaaa6372e95478610d7e08; // Bellatrix low
        dw 0x700000036fa50131482fe2af396daf2; // Bellatrix high

        dw 0x60f0a2ed78c1a85f0654941a0d19d0fa; // Capella low
        dw 0x700000047eb72b3be36f08feffcaba7; // Capella high

        dw 0x55fcf34b7e308f8fbca8e663bf565808; // Deneb low
        dw 0x7000000d31f6191ca65c836e170318c; // Deneb high

        dw 0x5b64eb2f9c81e0683f21dd0491e95aaa; // Electra low
        dw 0x700000014045b5a1d8da091c2ee9e63; // Electra high

    }

    func compute{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*}(network_id: felt, slot: felt) -> Uint256 {
        let fork_root = Network.get_fork_root(network_id, slot);

        // We now need to right right-shift the fork root 4 bytes, and prepend the domain
        let (q_high, r_high) = felt_divmod(fork_root.high, 0x100000000);
        let (q_low, _r_low) = felt_divmod(fork_root.low, 0x100000000);

        let high = DOMAIN_SYNC_COMMITTEE + q_high;
        let low = r_high * 0x1000000000000000000000000 + q_low;

        return (Uint256(low=low, high=high));
    }
}

// this function is used to compute the domain values found in get_domain()
// func precompute_domains{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, pow2_array: felt*, sha256_ptr: felt*}() {
//     alloc_locals;

//     // Precompute domains for MAINNET.
//     let (_, genesis_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.GENESIS);
//     let domain_mainnet_genesis = Domain.compute(Network.MAINNET, genesis_slot_mainnet);
//     print_string('domain_mainnet_genesis');
//     print_uint256(domain_mainnet_genesis);

//     let (_, altair_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.ALTAIR);
//     let domain_mainnet_altair = Domain.compute(Network.MAINNET, altair_slot_mainnet);
//     print_string('domain_mainnet_altair');
//     print_uint256(domain_mainnet_altair);

//     let (_, bellatrix_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.BELLATRIX);
//     let domain_mainnet_bellatrix = Domain.compute(Network.MAINNET, bellatrix_slot_mainnet);
//     print_string('domain_mainnet_bellatrix');
//     print_uint256(domain_mainnet_bellatrix);

//     let (_, capella_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.CAPELLA);
//     let domain_mainnet_capella = Domain.compute(Network.MAINNET, capella_slot_mainnet);
//     print_string('domain_mainnet_capella');
//     print_uint256(domain_mainnet_capella);

//     let (_, deneb_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.DENEB);
//     let domain_mainnet_deneb = Domain.compute(Network.MAINNET, deneb_slot_mainnet);
//     print_string('domain_mainnet_deneb');
//     print_uint256(domain_mainnet_deneb);

//     let (_, electra_slot_mainnet) = Network.get_fork_data(Network.MAINNET, Network.ELECTRA);
//     print_string('got fork data');
//     let domain_mainnet_electra = Domain.compute(Network.MAINNET, electra_slot_mainnet - 1);
//     print_string('domain_mainnet_electra');
//     print_uint256(domain_mainnet_electra);


//     // Precompute domains for SEPOLIA.
//     let (_, genesis_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.GENESIS);
//     let domain_sepolia_genesis = Domain.compute(Network.SEPOLIA, genesis_slot_sepolia);
//     print_string('domain_sepolia_genesis');
//     print_uint256(domain_sepolia_genesis);

//     let (_, altair_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.ALTAIR);
//     let domain_sepolia_altair = Domain.compute(Network.SEPOLIA, altair_slot_sepolia);
//     print_string('domain_sepolia_altair');
//     print_uint256(domain_sepolia_altair);

//     let (_, bellatrix_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.BELLATRIX);
//     let domain_sepolia_bellatrix = Domain.compute(Network.SEPOLIA, bellatrix_slot_sepolia);
//     print_string('domain_sepolia_bellatrix');
//     print_uint256(domain_sepolia_bellatrix);

//     let (_, capella_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.CAPELLA);
//     let domain_sepolia_capella = Domain.compute(Network.SEPOLIA, capella_slot_sepolia);
//     print_string('domain_sepolia_capella');
//     print_uint256(domain_sepolia_capella);

//     let (_, deneb_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.DENEB);
//     let domain_sepolia_deneb = Domain.compute(Network.SEPOLIA, deneb_slot_sepolia);
//     print_string('domain_sepolia_deneb');
//     print_uint256(domain_sepolia_deneb);

//     let (_, electra_slot_sepolia) = Network.get_fork_data(Network.SEPOLIA, Network.ELECTRA);
//     let domain_sepolia_electra = Domain.compute(Network.SEPOLIA, electra_slot_sepolia);
//     print_string('domain_sepolia_electra');
//     print_uint256(domain_sepolia_electra);

//     return ();

// }

// func print_uint256(value: Uint256) {
//     print_string('Uint256:');
//     print_felt_hex(value.low);
//     print_felt_hex(value.high);
    
//     return ();
// }
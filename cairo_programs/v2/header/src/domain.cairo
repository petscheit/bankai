mod signing_root {
    use header::primitives::types::{Hash, HashImpl};
    use header::domain::fork;
    use header::utils::print::ArrayPrintImpl;
    use header::merkle::pair_container;


    struct ForkVersion {
        version: Array<u8>,
        activation_epoch: u64,
    }

    const DOMAIN_SYNC_COMMITTEE: u32 = 0x07000000;

    // hash_tree_root fork version + 
    fn compute_fork_data_root(slot: u64) -> Array<u8> {
        let fork_version = fork::get_fork_version(slot);
        let genesis_validator_root: Array<u8> = array![
            0xd8, 0xea, 0x17, 0x1f, 0x3c, 0x94, 0xae, 0xa2, 
            0x1e, 0xbc, 0x42, 0xa1, 0xed, 0x61, 0x05, 0x2a, 
            0xcf, 0x3f, 0x92, 0x09, 0xc0, 0x0e, 0x4e, 0xfb, 
            0xaa, 0xdd, 0xac, 0x09, 0xed, 0x9b, 0x80, 0x78
        ];

        return pair_container::hash_tree_root(
            fork_version,
            genesis_validator_root
        );
    }

    fn compute_domain(slot: u64) -> Array<u8> {
        let fork_data_root = compute_fork_data_root(slot);

        let domain: Array<u8> = array![
            0x07,
            0x00,
            0x00,
            0x00, // first for bytes are signers domain -> sync committee in this case
            *fork_data_root.at(0),
            *fork_data_root.at(1),
            *fork_data_root.at(2),
            *fork_data_root.at(3),
            *fork_data_root.at(4),
            *fork_data_root.at(5),
            *fork_data_root.at(6),
            *fork_data_root.at(7),
            *fork_data_root.at(8),
            *fork_data_root.at(9),
            *fork_data_root.at(10),
            *fork_data_root.at(11),
            *fork_data_root.at(12),
            *fork_data_root.at(13),
            *fork_data_root.at(14),
            *fork_data_root.at(15),
            *fork_data_root.at(16),
            *fork_data_root.at(17),
            *fork_data_root.at(18),
            *fork_data_root.at(19),
            *fork_data_root.at(20),
            *fork_data_root.at(21),
            *fork_data_root.at(22),
            *fork_data_root.at(23),
            *fork_data_root.at(24),
            *fork_data_root.at(25),
            *fork_data_root.at(26),
            *fork_data_root.at(27), // the remaining bytes are filled with the MSB of the genesis validator root
        ];

        return domain;
    }

    fn compute(message: Array<u8>, slot: u64) -> Array<u8> {
        let domain = compute_domain(slot);

        return pair_container::hash_tree_root(
            message,
            domain
        );
    }
}

// handles the different fork versions, which is required for the domain computation
mod fork {
    const GENESIS_ACTIVATION_SLOT: u64 = 0; // 0 * SLOTS_PER_EPOCH;
    const ALTAIR_ACTIVATION_SLOT: u64 = 1600; // 50 * SLOTS_PER_EPOCH;
    const BELLATRIX_ACTIVATION_SLOT: u64 = 3200; // 100 * SLOTS_PER_EPOCH;
    const CAPPELLA_ACTIVATION_SLOT: u64 = 1818624; //56832 * SLOTS_PER_EPOCH;

    fn get_fork_version(slot: u64) -> Array<u8> {
        if slot < ALTAIR_ACTIVATION_SLOT {
            return array![
                0x90, 0x00, 0x00, 0x69, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ];
        } else if slot < BELLATRIX_ACTIVATION_SLOT {
            return array![
                0x90, 0x00, 0x00, 0x70, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ];
        } else if slot < CAPPELLA_ACTIVATION_SLOT {
            return array![
                0x90, 0x00, 0x00, 0x71, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ];
        } else {
            return array![
                0x90, 0x00, 0x00, 0x72, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
            ];
        }
    }
}



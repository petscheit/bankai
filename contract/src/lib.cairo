#[derive(Drop, starknet::Store, Serde)] // Added Serde trait
pub struct EpochProof {
    header_root: u256,
    state_root: u256,
    n_signers: u64,
}

#[starknet::interface]
pub trait IBankaiContract<TContractState> {
    fn get_committee_hash(self: @TContractState, committee_id: u64) -> u256;
    fn get_latest_epoch(self: @TContractState) -> u64;
    fn get_latest_committee_id(self: @TContractState) -> u64;
    fn get_committee_update_program_hash(self: @TContractState) -> felt252;
    fn get_epoch_update_program_hash(self: @TContractState) -> felt252;
    fn get_epoch_proof(self: @TContractState, slot: u64) -> EpochProof;
    fn verify_committee_update(
        ref self: TContractState, state_root: u256, committee_hash: u256, slot: u64,
    );
    fn verify_epoch_update(
        ref self: TContractState,
        header_root: u256,
        state_root: u256,
        committee_hash: u256,
        n_signers: u64,
        slot: u64,
    );
}

pub mod utils;
#[starknet::contract]
pub mod BankaiContract {
    use super::EpochProof;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::{ContractAddress, get_caller_address};
    use integrity::{Integrity, IntegrityWithConfig, SHARP_BOOTLOADER_PROGRAM_HASH, VerifierConfiguration};
    use crate::utils::{calculate_wrapped_bootloaded_fact_hash, WRAPPER_PROGRAM_HASH};
    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        CommitteeUpdated: CommitteeUpdated,
        // EpochUpdated: EpochUpdated,
    }

    #[derive(Drop, starknet::Event)]
    pub struct CommitteeUpdated {
        committee_id: u64,
        committee_hash: u256,
    }

    #[storage]
    struct Storage {
        committee: Map::<
            u64, u256,
        >, // maps committee index to committee hash (sha256(x || y)) of aggregate key
        epochs: Map::<u64, EpochProof>, // maps beacon slot to header root and state root
        owner: ContractAddress,
        latest_epoch: u64,
        latest_committee_id: u64,
        initialization_committee: u64,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
    }

    #[constructor]
    pub fn constructor(
        ref self: ContractState,
        committee_id: u64,
        committee_hash: u256,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
    ) {
        self.owner.write(get_caller_address());
        self.latest_epoch.write(0);

        // Write trusted initial committee
        self.initialization_committee.write(committee_id);
        self.latest_committee_id.write(committee_id);
        self.committee.write(committee_id, committee_hash);

        // Write the program hashes to the contract storage
        self.committee_update_program_hash.write(committee_update_program_hash);
        self.epoch_update_program_hash.write(epoch_update_program_hash);
    }

    #[abi(embed_v0)]
    impl BankaiContractImpl of super::IBankaiContract<ContractState> {
        fn get_committee_hash(self: @ContractState, committee_id: u64) -> u256 {
            self.committee.read(committee_id)
        }

        fn get_latest_epoch(self: @ContractState) -> u64 {
            self.latest_epoch.read()
        }

        fn get_latest_committee_id(self: @ContractState) -> u64 {
            self.latest_committee_id.read()
        }

        fn get_committee_update_program_hash(self: @ContractState) -> felt252 {
            self.committee_update_program_hash.read()
        }

        fn get_epoch_update_program_hash(self: @ContractState) -> felt252 {
            self.epoch_update_program_hash.read()
        }

        fn get_epoch_proof(self: @ContractState, slot: u64) -> EpochProof {
            self.epochs.read(slot)
        }

        fn verify_committee_update(
            ref self: ContractState, state_root: u256, committee_hash: u256, slot: u64,
        ) {
            let epoch_proof = self.epochs.read(slot);
            assert(state_root == epoch_proof.state_root, 'Invalid State Root!');

            // for now we dont ensure the fact hash is valid
            let fact_hash = compute_committee_proof_fact_hash(
                @self, state_root, committee_hash, slot,
            );
            // println!("fact_hash: {:?}", fact_hash);
            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            // The new committee is always assigned at the start of the previous committee
            let new_committee_id = (slot / 0x2000) + 1;

            self.committee.write(new_committee_id, committee_hash);
            self.latest_committee_id.write(new_committee_id);
            self
                .emit(
                    Event::CommitteeUpdated(
                        CommitteeUpdated {
                            committee_id: new_committee_id, committee_hash: committee_hash,
                        },
                    ),
                );
        }

        fn verify_epoch_update(
            ref self: ContractState,
            header_root: u256,
            state_root: u256,
            committee_hash: u256,
            n_signers: u64,
            slot: u64,
        ) {
            // println!("verify_epoch_update");
            let signing_committee_id = (slot / 0x2000);
            let valid_committee_hash = self.committee.read(signing_committee_id);
            assert(committee_hash == valid_committee_hash, 'Invalid Committee Hash!');

            let fact_hash = compute_epoch_proof_fact_hash(
                @self, header_root, state_root, committee_hash, n_signers, slot,
            );

            // println!("fact_hash: {:?}", fact_hash);
            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            let epoch_proof = EpochProof {
                header_root: header_root, state_root: state_root, n_signers: n_signers,
            };
            self.epochs.write(slot, epoch_proof);

            self.latest_epoch.write(slot);
            // self.emit(Event::EpochUpdated(EpochUpdated {
        //     slot: slot,
        //     epoch_proof: epoch_proof
        // }));
        }
    }

    fn compute_committee_proof_fact_hash(
        self: @ContractState, state_root: u256, committee_hash: u256, slot: u64,
    ) -> felt252 {
        let fact_hash = calculate_wrapped_bootloaded_fact_hash(
            WRAPPER_PROGRAM_HASH,
            SHARP_BOOTLOADER_PROGRAM_HASH,
            self.committee_update_program_hash.read(),
            [
                state_root.low.into(), state_root.high.into(), committee_hash.low.into(),
                committee_hash.high.into(), slot.into(),
            ]
                .span(),
        );
        return fact_hash;
    }

    fn compute_epoch_proof_fact_hash(
        self: @ContractState,
        header_root: u256,
        state_root: u256,
        committee_hash: u256,
        n_signers: u64,
        slot: u64,
    ) -> felt252 {
        let fact_hash = calculate_wrapped_bootloaded_fact_hash(
            WRAPPER_PROGRAM_HASH,
            SHARP_BOOTLOADER_PROGRAM_HASH,
            self.epoch_update_program_hash.read(),
            [
                header_root.low.into(), header_root.high.into(), state_root.low.into(),
                state_root.high.into(), committee_hash.low.into(), committee_hash.high.into(),
                n_signers.into(), slot.into(),
            ]
                .span(),
        );
        return fact_hash;
    }

    fn is_valid_fact_hash(fact_hash: felt252) -> bool {
        let config = VerifierConfiguration {
            layout: 'recursive_with_poseidon',
            hasher: 'keccak_160_lsb',
            stone_version: 'stone6',
            memory_verification: 'relaxed',
        };
        let SECURITY_BITS = 96;

        let integrity = Integrity::new().with_config(config, SECURITY_BITS);
        integrity.is_fact_hash_valid(fact_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::{BankaiContract, IBankaiContract};
    use starknet::testing::{set_caller_address};
    use starknet::ContractAddress;

    // Add this struct and function before your test functions
    #[derive(Drop, Clone)]
    struct TestFixture {
        committee_id: u64,
        committee_hash: u256,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
        owner: ContractAddress,
    }

    #[derive(Drop, Clone)]
    struct EpochUpdateFixture {
        header_root: u256,
        state_root: u256,
        committee_hash: u256,
        n_signers: u64,
        slot: u64,
    }

    #[derive(Drop, Clone)]
    struct CommitteeUpdateFixture {
        slot: u64,
        state_root: u256,
        new_committee_hash: u256,
        expected_committee_id: u64,
    }

    fn setup_fixture() -> TestFixture {
        TestFixture {
            committee_id: 1,
            committee_hash: 0x111_u256,
            committee_update_program_hash: 0xdead_felt252,
            epoch_update_program_hash: 0xbeef_felt252,
            owner: starknet::contract_address_const::<0x123>(),
        }
    }

    fn get_epoch_update_fixture(index: u8) -> EpochUpdateFixture {
        match index {
            0 => EpochUpdateFixture {
                header_root: 0xbbb_u256,
                state_root: 0xbbb_u256,
                committee_hash: 0x111_u256, // Matches initial committee hash from setup_fixture
                n_signers: 512,
                slot: 8192 // First term
            },
            1 => EpochUpdateFixture {
                header_root: 0xccc_u256,
                state_root: 0xccc_u256,
                committee_hash: 0x222_u256, // Matches new committee hash from committee update fixture
                n_signers: 384,
                slot: 16384 // Second term
            },
            _ => EpochUpdateFixture {
                header_root: 0xddd_u256,
                state_root: 0xddd_u256,
                committee_hash: 0x333_u256,
                n_signers: 256,
                slot: 24576 // Third term
            },
        }
    }

    fn get_committee_update_fixture(index: u8) -> CommitteeUpdateFixture {
        match index {
            0 => CommitteeUpdateFixture {
                slot: 8192, // First term
                state_root: 0xbbb_u256,
                new_committee_hash: 0x222_u256,
                expected_committee_id: 2 // slot/0x2000 + 1
            },
            _ => CommitteeUpdateFixture {
                slot: 16384, // Second term
                state_root: 0xccc_u256,
                new_committee_hash: 0x333_u256,
                expected_committee_id: 3 // slot/0x2000 + 1
            },
        }
    }

    // Helper function to deploy the contract
    fn deploy(fixture: TestFixture) -> BankaiContract::ContractState {
        let mut state = BankaiContract::contract_state_for_testing();
        BankaiContract::constructor(
            ref state,
            fixture.committee_id,
            fixture.committee_hash,
            fixture.committee_update_program_hash,
            fixture.epoch_update_program_hash,
            // fixture.init_slot,
        // fixture.init_header_root,
        // fixture.init_state_root,
        // fixture.init_n_signers,
        );
        state
    }

    // Example of how to update your test to use the fixture
    #[test]
    fn test_deploy() {
        let fixture = setup_fixture();
        set_caller_address(fixture.owner);

        let state = deploy(fixture.clone());

        assert_eq!(state.get_committee_hash(fixture.committee_id), fixture.committee_hash);
        assert_eq!(state.get_latest_epoch(), 0);
        assert_eq!(
            state.get_committee_update_program_hash(), fixture.committee_update_program_hash,
        );
        assert_eq!(state.get_epoch_update_program_hash(), fixture.epoch_update_program_hash);
    }

    #[test]
    fn test_epoch_update_with_fixtures() {
        let fixture = setup_fixture();
        let epoch_fixture = get_epoch_update_fixture(0); // Use first fixture
        set_caller_address(fixture.owner);

        let mut state = deploy(fixture.clone());
        println!("deploy success");

        // First update should succeed
        state
            .verify_epoch_update(
                epoch_fixture.header_root,
                epoch_fixture.state_root,
                epoch_fixture.committee_hash,
                epoch_fixture.n_signers,
                epoch_fixture.slot,
            );

        // Verify the epoch was stored correctly
        let stored_epoch = state.get_epoch_proof(epoch_fixture.slot);
        assert_eq!(stored_epoch.header_root, epoch_fixture.header_root);
        assert_eq!(stored_epoch.state_root, epoch_fixture.state_root);
        assert_eq!(stored_epoch.n_signers, epoch_fixture.n_signers);

        println!("first epoch update success");

        let committee_update = get_committee_update_fixture(0);
        state
            .verify_committee_update(
                committee_update.state_root,
                committee_update.new_committee_hash,
                committee_update.slot,
            );

        println!("committee update success");

        assert_eq!(
            state.get_committee_hash(committee_update.expected_committee_id),
            committee_update.new_committee_hash,
        );

        let epoch_fixture = get_epoch_update_fixture(1);
        state
            .verify_epoch_update(
                epoch_fixture.header_root,
                epoch_fixture.state_root,
                epoch_fixture.committee_hash,
                epoch_fixture.n_signers,
                epoch_fixture.slot,
            );

        println!("second epoch update success");

        assert_eq!(
            state.get_committee_hash(committee_update.expected_committee_id),
            committee_update.new_committee_hash,
        );
    }
}

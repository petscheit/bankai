#[derive(Drop, starknet::Store, Serde)]
pub struct EpochProof {
    // Hash of the beacon header (root since ssz)
    header_root: u256,
    // state root at the mapped slot
    beacon_state_root: u256,
    // Number of signers (out of 512)
    n_signers: u64,
    // Hash of the execution header
    execution_hash: u256,
    // Height of the execution header
    execution_height: u64,
}

#[starknet::interface]
pub trait IBankaiContract<TContractState> {
    fn get_committee_hash(self: @TContractState, committee_id: u64) -> u256;
    fn get_latest_epoch_slot(self: @TContractState) -> u64;
    fn get_latest_committee_id(self: @TContractState) -> u64;
    fn get_committee_update_program_hash(self: @TContractState) -> felt252;
    fn get_epoch_update_program_hash(self: @TContractState) -> felt252;
    fn get_epoch_proof(self: @TContractState, slot: u64) -> EpochProof;
    fn verify_committee_update(
        ref self: TContractState, beacon_state_root: u256, committee_hash: u256, slot: u64,
    );
    fn verify_epoch_update(
        ref self: TContractState,
        header_root: u256,
        beacon_state_root: u256,
        slot: u64,
        committee_hash: u256,
        n_signers: u64,
        execution_hash: u256,
        execution_height: u64,
    );

    fn verify_epoch_batch(
        ref self: TContractState,
        batch_root: felt252,
        header_root: u256,
        beacon_state_root: u256,
        slot: u64,
        committee_hash: u256,
        n_signers: u64,
        execution_hash: u256,
        execution_height: u64,
    );

    fn decommit_batched_epoch(
        ref self: TContractState,
        batch_root: felt252,
        merkle_index: u16,
        merkle_path: Array<felt252>,
        header_root: u256,
        beacon_state_root: u256,
        slot: u64,
        committee_hash: u256,
        n_signers: u64,
        execution_hash: u256,
        execution_height: u64,
    );

    fn propose_program_hash_update(
        ref self: TContractState,
        new_committee_hash: felt252,
        new_epoch_hash: felt252,
        new_batch_hash: felt252
    );
    fn execute_program_hash_update(ref self: TContractState);
    fn pause(ref self: TContractState);
    fn unpause(ref self: TContractState);
    fn is_paused(self: @TContractState) -> bool;
}

pub mod utils;
#[starknet::contract]
pub mod BankaiContract {
    use super::EpochProof;
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::{ContractAddress, get_caller_address, get_block_timestamp};
    use integrity::{
        Integrity, IntegrityWithConfig, SHARP_BOOTLOADER_PROGRAM_HASH, VerifierConfiguration,
    };
    use crate::utils::{calculate_wrapped_bootloaded_fact_hash, WRAPPER_PROGRAM_HASH, hash_path, compute_leaf_hash};
    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        CommitteeUpdated: CommitteeUpdated,
        EpochUpdated: EpochUpdated,
        EpochBatch: EpochBatch,
        EpochDecommitted: EpochDecommitted,
        Paused: Paused,
        Unpaused: Unpaused,
    }

    #[derive(Drop, starknet::Event)]
    pub struct CommitteeUpdated {
        committee_id: u64,
        committee_hash: u256,
    }

    #[derive(Drop, starknet::Event)]
    pub struct EpochUpdated {
        // Hash of the beacon header (root since ssz)
        beacon_root: u256,
        // Slot of the beacon header
        slot: u64,
        // Hash of the execution header
        execution_hash: u256,
        // Height of the execution header
        execution_height: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct EpochBatch {
        batch_root: felt252,
        beacon_root: u256,
        slot: u64,
        execution_hash: u256,
        execution_height: u64,
    }

    #[derive(Drop, starknet::Event)]
    pub struct EpochDecommitted {
        batch_root: felt252,
        slot: u64,
        execution_hash: u256,
        execution_height: u64
    }

    /// Emitted when the contract is paused
    #[derive(Drop, starknet::Event)]
    pub struct Paused {}

    /// Emitted when the contract is unpaused
    #[derive(Drop, starknet::Event)]
    pub struct Unpaused {}

    /// Time delay required for program hash updates (48 hours in seconds)
    const UPDATE_DELAY: u64 = 172800;

    #[storage]
    struct Storage {
        // Committee Management
        committee: Map::<u64, u256>, // Maps committee index to committee hash (sha256(x || y)) of aggregate key
        latest_committee_id: u64,
        initialization_committee: u64,

        // Epoch Management
        epochs: Map::<u64, EpochProof>, // Maps beacon slot to header root and state root
        latest_epoch_slot: u64,
        
        // Batch Management
        batches: Map::<felt252, bool>, // Tracks verified batch roots
        
        // Program Hash Management
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
        epoch_batch_program_hash: felt252,
        pending_committee_program_hash: felt252,
        pending_epoch_program_hash: felt252,
        pending_batch_program_hash: felt252,
        pending_update_timestamp: u64,
        
        // Access Control
        owner: ContractAddress,
        paused: bool,
    }

    #[constructor]
    pub fn constructor(
        ref self: ContractState,
        committee_id: u64,
        committee_hash: u256,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
        epoch_batch_program_hash: felt252,
    ) {
        self.owner.write(get_caller_address());
        self.latest_epoch_slot.write(0);

        // Write trusted initial committee
        self.initialization_committee.write(committee_id);
        self.latest_committee_id.write(committee_id);
        self.committee.write(committee_id, committee_hash);

        // Write the program hashes to the contract storage
        self.committee_update_program_hash.write(committee_update_program_hash);
        self.epoch_update_program_hash.write(epoch_update_program_hash);
        self.epoch_batch_program_hash.write(epoch_batch_program_hash);
    }

    #[abi(embed_v0)]
    impl BankaiContractImpl of super::IBankaiContract<ContractState> {
        fn get_committee_hash(self: @ContractState, committee_id: u64) -> u256 {
            self.committee.read(committee_id)
        }

        fn get_latest_epoch_slot(self: @ContractState) -> u64 {
            self.latest_epoch_slot.read()
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
            ref self: ContractState, beacon_state_root: u256, committee_hash: u256, slot: u64,
        ) {
            assert(!self.paused.read(), 'Contract is paused');
            let epoch_proof = self.epochs.read(slot);
            assert(beacon_state_root == epoch_proof.beacon_state_root, 'Invalid State Root!');

            let fact_hash = compute_committee_proof_fact_hash(
                @self, beacon_state_root, committee_hash, slot,
            );
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
            beacon_state_root: u256,
            slot: u64,
            committee_hash: u256,
            n_signers: u64,
            execution_hash: u256,
            execution_height: u64,
        ) {
            assert(!self.paused.read(), 'Contract is paused');
            let signing_committee_id = (slot / 0x2000);
            let valid_committee_hash = self.committee.read(signing_committee_id);
            assert(committee_hash == valid_committee_hash, 'Invalid Committee Hash!');

            let fact_hash = compute_epoch_proof_fact_hash(
                @self, header_root, beacon_state_root, slot, committee_hash, n_signers, execution_hash, execution_height,
            );

            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            let epoch_proof = EpochProof {
                header_root: header_root, beacon_state_root: beacon_state_root, n_signers: n_signers, execution_hash: execution_hash, execution_height: execution_height,
            };
            self.epochs.write(slot, epoch_proof);

            let latest_epoch = self.latest_epoch_slot.read();
            if slot > latest_epoch {
                self.latest_epoch_slot.write(slot);
            }

            self.emit(Event::EpochUpdated(EpochUpdated {
                beacon_root: header_root, slot: slot, execution_hash: execution_hash, execution_height: execution_height,
            }));
        }

        fn verify_epoch_batch(
            ref self: ContractState,
            batch_root: felt252,
            header_root: u256,
            beacon_state_root: u256,
            slot: u64,
            committee_hash: u256,
            n_signers: u64,
            execution_hash: u256,
            execution_height: u64,
        ) { 
            assert(!self.paused.read(), 'Contract is paused');

            let signing_committee_id = (slot / 0x2000);
            let valid_committee_hash = self.committee.read(signing_committee_id);
            assert(committee_hash == valid_committee_hash, 'Invalid Committee Hash!');

            let fact_hash = compute_epoch_batch_fact_hash(
                @self, batch_root, header_root, beacon_state_root, slot, committee_hash, n_signers, execution_hash, execution_height,
            );

            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            let epoch_proof = EpochProof {
                header_root: header_root, beacon_state_root: beacon_state_root, n_signers: n_signers, execution_hash: execution_hash, execution_height: execution_height,
            };
            self.epochs.write(slot, epoch_proof);

            self.emit(Event::EpochBatch(EpochBatch {
                batch_root: batch_root, beacon_root: header_root, slot: slot, execution_hash: execution_hash, execution_height: execution_height,
            }));

            self.batches.write(batch_root, true);

            let latest_epoch = self.latest_epoch_slot.read();
            if slot > latest_epoch {
                self.latest_epoch_slot.write(slot);
            }
        }

        fn decommit_batched_epoch(
            ref self: ContractState,
            batch_root: felt252,
            merkle_index: u16,
            merkle_path: Array<felt252>,
            header_root: u256,
            beacon_state_root: u256,
            slot: u64,
            committee_hash: u256,
            n_signers: u64,
            execution_hash: u256,
            execution_height: u64,
        ) {
            assert(!self.paused.read(), 'Contract is paused');
            let known_batch_root = self.batches.read(batch_root);
            assert(known_batch_root, 'Batch root not known!');

            let leaf = compute_leaf_hash(header_root, beacon_state_root, slot, committee_hash, n_signers, execution_hash, execution_height);

            let computed_root = hash_path(leaf, merkle_path, merkle_index);
            assert(computed_root == batch_root, 'Invalid Batch Merkle Root!');

            let epoch_proof = EpochProof {
                header_root: header_root, beacon_state_root: beacon_state_root, n_signers: n_signers, execution_hash: execution_hash, execution_height: execution_height,
            };
            self.epochs.write(slot, epoch_proof);
            
            self.emit(Event::EpochDecommitted(EpochDecommitted {
                batch_root: batch_root, slot: slot, execution_hash: execution_hash, execution_height: execution_height,
            }));
        }
        
        fn propose_program_hash_update(
            ref self: ContractState,
            new_committee_hash: felt252,
            new_epoch_hash: felt252,
            new_batch_hash: felt252
        ) {
            assert(!self.paused.read(), 'Contract is paused');
            assert(get_caller_address() == self.owner.read(), 'Caller is not owner');
            
            self.pending_committee_program_hash.write(new_committee_hash);
            self.pending_epoch_program_hash.write(new_epoch_hash);
            self.pending_batch_program_hash.write(new_batch_hash);
            self.pending_update_timestamp.write(get_block_timestamp() + UPDATE_DELAY);
        }

        fn execute_program_hash_update(ref self: ContractState) {
            assert(get_caller_address() == self.owner.read(), 'Caller is not owner');
            assert(get_block_timestamp() >= self.pending_update_timestamp.read(), 'Delay not elapsed');
            
            // Update program hashes
            self.committee_update_program_hash.write(self.pending_committee_program_hash.read());
            self.epoch_update_program_hash.write(self.pending_epoch_program_hash.read());
            self.epoch_batch_program_hash.write(self.pending_batch_program_hash.read());

            // Clear pending updates
            self.pending_committee_program_hash.write(0);
            self.pending_epoch_program_hash.write(0);
            self.pending_batch_program_hash.write(0);
            self.pending_update_timestamp.write(0);
        }

        fn pause(ref self: ContractState) {
            assert(get_caller_address() == self.owner.read(), 'Caller is not owner');
            assert(!self.paused.read(), 'Contract is already paused');
            self.paused.write(true);
            self.emit(Event::Paused(Paused {}));
        }

        fn unpause(ref self: ContractState) {
            assert(get_caller_address() == self.owner.read(), 'Caller is not owner');
            assert(self.paused.read(), 'Contract is not paused');
            self.paused.write(false);
            self.emit(Event::Unpaused(Unpaused {}));
        }

        fn is_paused(self: @ContractState) -> bool {
            self.paused.read()
        }
    }

    /// Internal helper functions for computing fact hashes
    fn compute_committee_proof_fact_hash(
        self: @ContractState, beacon_state_root: u256, committee_hash: u256, slot: u64,
    ) -> felt252 {
        let fact_hash = calculate_wrapped_bootloaded_fact_hash(
            WRAPPER_PROGRAM_HASH,
            SHARP_BOOTLOADER_PROGRAM_HASH,
            self.committee_update_program_hash.read(),
            [
                beacon_state_root.low.into(), beacon_state_root.high.into(), committee_hash.low.into(),
                committee_hash.high.into(), slot.into(),
            ]
                .span(),
        );
        return fact_hash;
    }

    /// Computes fact hash for epoch proof verification
    fn compute_epoch_proof_fact_hash(
        self: @ContractState,
        header_root: u256,
        state_root: u256,
        slot: u64,
        committee_hash: u256,
        n_signers: u64,
        execution_hash: u256,
        execution_height: u64,
    ) -> felt252 {
        let fact_hash = calculate_wrapped_bootloaded_fact_hash(
            WRAPPER_PROGRAM_HASH,
            SHARP_BOOTLOADER_PROGRAM_HASH,
            self.epoch_update_program_hash.read(),
            [
                header_root.low.into(), header_root.high.into(), state_root.low.into(),
                state_root.high.into(), slot.into(), committee_hash.low.into(),
                committee_hash.high.into(), n_signers.into(), execution_hash.low.into(),
                execution_hash.high.into(), execution_height.into(),
            ]
                .span(),
        );
        return fact_hash;
    }

    /// Computes fact hash for epoch batch verification
    fn compute_epoch_batch_fact_hash(
        self: @ContractState,
        batch_root: felt252,
        header_root: u256,
        state_root: u256,
        slot: u64,
        committee_hash: u256,
        n_signers: u64,
        execution_hash: u256,
        execution_height: u64,
    ) -> felt252 {
        let fact_hash = calculate_wrapped_bootloaded_fact_hash(
            WRAPPER_PROGRAM_HASH,
            SHARP_BOOTLOADER_PROGRAM_HASH,
            self.epoch_batch_program_hash.read(),
            [
                batch_root, header_root.low.into(),
                header_root.high.into(), state_root.low.into(),
                state_root.high.into(), slot.into(), committee_hash.low.into(),
                committee_hash.high.into(), n_signers.into(), execution_hash.low.into(),
                execution_hash.high.into(), execution_height.into(),
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
    use super::BankaiContract;
    use super::IBankaiContract;
    use starknet::contract_address_const;
    use starknet::testing::set_caller_address;
    use starknet::testing::set_block_timestamp;

    // Helper function to deploy the contract for testing
    fn deploy_contract() -> BankaiContract::ContractState {
        let mut state = BankaiContract::contract_state_for_testing();
        
        // Set caller as contract deployer
        set_caller_address(contract_address_const::<0x123>());
        
        // Initialize with some test values
        BankaiContract::constructor(
            ref state,
            1, // committee_id
            1234.into(), // committee_hash
            111.into(), // committee_update_program_hash
            222.into(), // epoch_update_program_hash
            333.into() // epoch_batch_program_hash
        );
        
        state
    }

    #[test]
    fn test_constructor() {
        let state = deploy_contract();
        
        assert!(!IBankaiContract::is_paused(@state));
        assert_eq!(IBankaiContract::get_latest_epoch_slot(@state), 0);
        assert_eq!(IBankaiContract::get_latest_committee_id(@state), 1);
        assert_eq!(IBankaiContract::get_committee_hash(@state, 1), 1234.into());
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 111);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 222);
    }

    #[test]
    fn test_pause_unpause() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);
        
        // Test initial state
        assert!(!IBankaiContract::is_paused(@state));
        
        // Test pause
        IBankaiContract::pause(ref state);
        assert!(IBankaiContract::is_paused(@state));
        
        // Test unpause
        IBankaiContract::unpause(ref state);
        assert!(!IBankaiContract::is_paused(@state));
    }

    #[test]
    #[should_panic(expected: ('Caller is not owner',))]
    fn test_pause_unauthorized() {
        let mut state = deploy_contract();
        
        // Try to pause from different address
        let other = contract_address_const::<0x456>();
        set_caller_address(other);
        IBankaiContract::pause(ref state);
    }

    #[test]
    fn test_program_hash_update() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);
        
        // Set initial timestamp
        set_block_timestamp(1000);
        
        // Propose update
        IBankaiContract::propose_program_hash_update(
            ref state,
            444.into(), // new_committee_hash
            555.into(), // new_epoch_hash
            666.into()  // new_batch_hash
        );
        
        // Execute after delay
        set_block_timestamp(1000 + 172800); // After delay
        IBankaiContract::execute_program_hash_update(ref state);
        
        // Verify updates
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 444);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 555);
    }

    #[test]
    #[should_panic(expected: ('Delay not elapsed',))]
    fn test_program_hash_update_too_early() {
        let mut state = deploy_contract();
        let owner = contract_address_const::<0x123>();
        set_caller_address(owner);
        
        // Set initial timestamp
        set_block_timestamp(1000);
        
        // Propose update
        IBankaiContract::propose_program_hash_update(
            ref state,
            444.into(), // new_committee_hash
            555.into(), // new_epoch_hash
            666.into()  // new_batch_hash
        );
        
        // Try to execute before delay
        set_block_timestamp(1000 + 172799); // Just before delay
        IBankaiContract::execute_program_hash_update(ref state);
    }

    #[test]
    fn test_getters() {
        let state = deploy_contract();
        
        assert_eq!(IBankaiContract::get_committee_hash(@state, 1), 1234.into());
        assert_eq!(IBankaiContract::get_latest_epoch_slot(@state), 0);
        assert_eq!(IBankaiContract::get_latest_committee_id(@state), 1);
        assert_eq!(IBankaiContract::get_committee_update_program_hash(@state), 111);
        assert_eq!(IBankaiContract::get_epoch_update_program_hash(@state), 222);
    }
}
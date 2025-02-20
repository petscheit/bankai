pub mod interface;
pub mod types;

pub use interface::IBankaiContract;

pub mod utils;
#[starknet::contract]
pub mod BankaiContract {
    use super::types::{
        EpochProof, CommitteeUpdated, EpochUpdated, EpochBatch, EpochDecommitted, Paused,
        Unpaused,
    };
    use starknet::storage::{
        Map, StorageMapReadAccess, StorageMapWriteAccess, StoragePointerReadAccess,
        StoragePointerWriteAccess,
    };
    use starknet::ClassHash;

    use starknet::{ContractAddress, get_block_timestamp};
    use integrity::{
        Integrity, IntegrityWithConfig, SHARP_BOOTLOADER_PROGRAM_HASH, VerifierConfiguration,
    };
    use crate::utils::{
        calculate_wrapped_bootloaded_fact_hash, WRAPPER_PROGRAM_HASH, hash_path, compute_leaf_hash,
    };

    use openzeppelin_access::ownable::OwnableComponent;
    use openzeppelin_upgrades::UpgradeableComponent;
    use openzeppelin_upgrades::interface::IUpgradeable;

    component!(path: OwnableComponent, storage: ownable, event: OwnableEvent);
    component!(path: UpgradeableComponent, storage: upgradeable, event: UpgradeableEvent);


    // Ownable Mixin
    #[abi(embed_v0)]
    impl OwnableMixinImpl = OwnableComponent::OwnableMixinImpl<ContractState>;
    impl OwnableInternalImpl = OwnableComponent::InternalImpl<ContractState>;

    impl UpgradeableInternalImpl = UpgradeableComponent::InternalImpl<ContractState>;


    /// Events emitted by the contract
    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        /// Emitted when a new validator committee is verified
        CommitteeUpdated: CommitteeUpdated,
        /// Emitted when a new epoch is verified
        EpochUpdated: EpochUpdated,
        /// Emitted when a batch of epochs is verified
        EpochBatch: EpochBatch,
        /// Emitted when an epoch is extracted from a batch
        EpochDecommitted: EpochDecommitted,
        /// Emitted when the contract is paused
        Paused: Paused,
        /// Emitted when the contract is unpaused
        Unpaused: Unpaused,
        OwnableEvent: OwnableComponent::Event,
        UpgradeableEvent: UpgradeableComponent::Event,
    }

    /// Time delay required for program hash updates (48 hours in seconds)
    /// This delay provides a security window for detecting malicious updates
    const UPDATE_DELAY: u64 = 172800;

    /// Contract storage layout
    #[storage]
    struct Storage {
        // Committee Management
        /// Maps committee index to committee hash (sha256(x || y)) of aggregate key
        committee: Map::<u64, u256>,
        /// ID of the most recent committee
        latest_committee_id: u64,
        /// ID of the initial trusted committee
        initialization_committee: u64,
        // Epoch Management
        /// Maps beacon slot to header root and state root
        epochs: Map::<u64, EpochProof>,
        /// Most recent verified epoch slot
        latest_epoch_slot: u64,
        // Batch Management
        /// Tracks verified batch roots
        batches: Map::<felt252, bool>,
        // Program Hash Management
        /// Current SHARP program hash for committee updates
        committee_update_program_hash: felt252,
        /// Current SHARP program hash for epoch updates
        epoch_update_program_hash: felt252,
        /// Current SHARP program hash for epoch batching
        epoch_batch_program_hash: felt252,
        /// Proposed new committee program hash (pending timelock)
        pending_committee_program_hash: felt252,
        /// Proposed new epoch program hash (pending timelock)
        pending_epoch_program_hash: felt252,
        /// Proposed new batch program hash (pending timelock)
        pending_batch_program_hash: felt252,
        /// Timestamp when pending program hash update can be executed
        pending_update_timestamp: u64,
        // Contract Management
        /// Contract pause state for emergency stops
        paused: bool,
        /// OpenZeppelin ownable component storage
        #[substorage(v0)]
        pub ownable: OwnableComponent::Storage,
        /// OpenZeppelin upgradeable component storage
        #[substorage(v0)]
        upgradeable: UpgradeableComponent::Storage,
    }

    /// Contract constructor
    /// @param committee_id - ID of the initial trusted committee
    /// @param committee_hash - Hash of the initial committee's public key
    /// @param committee_update_program_hash - Initial SHARP program hash for committee updates
    /// @param epoch_update_program_hash - Initial SHARP program hash for epoch updates
    /// @param epoch_batch_program_hash - Initial SHARP program hash for epoch batching
    #[constructor]
    pub fn constructor(
        ref self: ContractState,
        committee_id: u64,
        committee_hash: u256,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
        epoch_batch_program_hash: felt252,
        owner: ContractAddress,
    ) {
        // Initialize owner as contract deployer
        self.ownable.initializer(owner);
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

    /// Implementation of the upgradeable interface
    #[abi(embed_v0)]
    impl UpgradeableImpl of IUpgradeable<ContractState> {
        /// Upgrades the contract to a new implementation
        /// @param new_class_hash - The class hash of the new implementation
        /// @dev Can only be called by the contract owner
        fn upgrade(ref self: ContractState, new_class_hash: ClassHash) {
            self.ownable.assert_only_owner();
            self.upgradeable.upgrade(new_class_hash);
        }
    }

    /// Core implementation of the Bankai contract interface
    #[abi(embed_v0)]
    impl BankaiContractImpl of super::IBankaiContract<ContractState> {
        /// Retrieves the hash of a specific validator committee
        /// @param committee_id - The unique identifier of the committee
        /// @return The aggregate public key hash of the committee
        fn get_committee_hash(self: @ContractState, committee_id: u64) -> u256 {
            self.committee.read(committee_id)
        }

        /// Returns the slot number of the most recent verified epoch
        fn get_latest_epoch_slot(self: @ContractState) -> u64 {
            self.latest_epoch_slot.read()
        }

        /// Returns the ID of the most recent validator committee
        fn get_latest_committee_id(self: @ContractState) -> u64 {
            self.latest_committee_id.read()
        }

        /// Returns the current SHARP program hash for committee updates
        fn get_committee_update_program_hash(self: @ContractState) -> felt252 {
            self.committee_update_program_hash.read()
        }

        /// Returns the current SHARP program hash for epoch updates
        fn get_epoch_update_program_hash(self: @ContractState) -> felt252 {
            self.epoch_update_program_hash.read()
        }

        /// Returns the current SHARP program hash for epoch batching
        fn get_epoch_batch_program_hash(self: @ContractState) -> felt252 {
            self.epoch_batch_program_hash.read()
        }

        /// Retrieves the epoch proof for a given slot
        /// @param slot - The slot number to query
        /// @return The epoch proof containing consensus and execution data
        fn get_epoch_proof(self: @ContractState, slot: u64) -> EpochProof {
            self.epochs.read(slot)
        }

        /// Verifies and stores a new validator committee update
        /// @dev Requires a valid SHARP proof and matching beacon state root
        /// @param beacon_state_root - The beacon chain state root containing the committee
        /// @param committee_hash - Hash of the new committee's public key
        /// @param slot - Slot number where this committee becomes active
        /// @custom:throws 'Contract is paused' if contract is paused
        /// @custom:throws 'Invalid State Root!' if beacon state root doesn't match
        /// @custom:throws 'Invalid Fact Hash!' if SHARP proof is invalid
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

        /// Verifies and stores a new epoch update
        /// @dev Requires a valid SHARP proof and matching committee hash
        /// @custom:throws 'Contract is paused' if contract is paused
        /// @custom:throws 'Invalid Committee Hash!' if committee hash doesn't match
        /// @custom:throws 'Invalid Fact Hash!' if SHARP proof is invalid
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
                @self,
                header_root,
                beacon_state_root,
                slot,
                committee_hash,
                n_signers,
                execution_hash,
                execution_height,
            );

            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            let epoch_proof = EpochProof {
                header_root, beacon_state_root, n_signers, execution_hash, execution_height,
            };
            self.epochs.write(slot, epoch_proof);

            let latest_epoch = self.latest_epoch_slot.read();
            if slot > latest_epoch {
                self.latest_epoch_slot.write(slot);
            }

            self
                .emit(
                    Event::EpochUpdated(
                        EpochUpdated {
                            beacon_root: header_root, slot, execution_hash, execution_height,
                        },
                    ),
                );
        }

        /// Verifies and stores a batch of epoch updates
        /// @dev Requires a valid SHARP proof and matching committee hash
        /// @custom:throws 'Contract is paused' if contract is paused
        /// @custom:throws 'Invalid Committee Hash!' if committee hash doesn't match
        /// @custom:throws 'Invalid Fact Hash!' if SHARP proof is invalid
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
                @self,
                batch_root,
                header_root,
                beacon_state_root,
                slot,
                committee_hash,
                n_signers,
                execution_hash,
                execution_height,
            );

            assert(is_valid_fact_hash(fact_hash), 'Invalid Fact Hash!');

            let epoch_proof = EpochProof {
                header_root, beacon_state_root, n_signers, execution_hash, execution_height,
            };
            self.epochs.write(slot, epoch_proof);

            self
                .emit(
                    Event::EpochBatch(
                        EpochBatch {
                            batch_root,
                            beacon_root: header_root,
                            slot,
                            execution_hash,
                            execution_height,
                        },
                    ),
                );

            self.batches.write(batch_root, true);

            let latest_epoch = self.latest_epoch_slot.read();
            if slot > latest_epoch {
                self.latest_epoch_slot.write(slot);
            }
        }

        /// Extracts and verifies a single epoch from a previously verified batch
        /// @dev Verifies the Merkle proof against the stored batch root
        /// @custom:throws 'Contract is paused' if contract is paused
        /// @custom:throws 'Batch root not known!' if batch_root hasn't been verified
        /// @custom:throws 'Invalid Batch Merkle Root!' if Merkle proof is invalid
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

            let leaf = compute_leaf_hash(
                header_root,
                beacon_state_root,
                slot,
                committee_hash,
                n_signers,
                execution_hash,
                execution_height,
            );

            let computed_root = hash_path(leaf, merkle_path, merkle_index);
            assert(computed_root == batch_root, 'Invalid Batch Merkle Root!');

            let epoch_proof = EpochProof {
                header_root, beacon_state_root, n_signers, execution_hash, execution_height,
            };
            self.epochs.write(slot, epoch_proof);

            self
                .emit(
                    Event::EpochDecommitted(
                        EpochDecommitted { batch_root, slot, execution_hash, execution_height },
                    ),
                );
        }

        /// Proposes an update to the SHARP program hashes
        /// @dev Requires owner access and initiates the timelock period
        /// @param new_committee_hash - New program hash for committee verification
        /// @param new_epoch_hash - New program hash for epoch verification
        /// @param new_batch_hash - New program hash for batch verification
        /// @custom:throws 'Contract is paused' if contract is paused
        fn propose_program_hash_update(
            ref self: ContractState,
            new_committee_hash: felt252,
            new_epoch_hash: felt252,
            new_batch_hash: felt252,
        ) {
            assert(!self.paused.read(), 'Contract is paused');
            self.ownable.assert_only_owner();

            self.pending_committee_program_hash.write(new_committee_hash);
            self.pending_epoch_program_hash.write(new_epoch_hash);
            self.pending_batch_program_hash.write(new_batch_hash);
            self.pending_update_timestamp.write(get_block_timestamp() + UPDATE_DELAY);
        }

        /// Executes a proposed program hash update after timelock expires
        /// @dev Can only be called by owner after timelock period
        /// @custom:throws 'Delay not elapsed' if timelock period hasn't passed
        fn execute_program_hash_update(ref self: ContractState) {
            self.ownable.assert_only_owner();
            assert(
                get_block_timestamp() >= self.pending_update_timestamp.read(), 'Delay not elapsed',
            );

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

        /// Pauses all contract operations
        /// @dev Can only be called by owner
        /// @custom:throws 'Contract is already paused' if already paused
        fn pause(ref self: ContractState) {
            self.ownable.assert_only_owner();
            assert(!self.paused.read(), 'Contract is already paused');
            self.paused.write(true);
            self.emit(Event::Paused(Paused {}));
        }

        /// Unpauses contract operations
        /// @dev Can only be called by owner
        /// @custom:throws 'Contract is not paused' if not paused
        fn unpause(ref self: ContractState) {
            self.ownable.assert_only_owner();
            assert(self.paused.read(), 'Contract is not paused');
            self.paused.write(false);
            self.emit(Event::Unpaused(Unpaused {}));
        }

        /// Returns whether the contract is currently paused
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
                beacon_state_root.low.into(), beacon_state_root.high.into(),
                committee_hash.low.into(), committee_hash.high.into(), slot.into(),
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
                batch_root, header_root.low.into(), header_root.high.into(), state_root.low.into(),
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

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
    fn get_latest_epoch(self: @TContractState) -> u64;
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
        EpochDecommitted: EpochDecommitted
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

    #[storage]
    struct Storage {
        committee: Map::<
            u64, u256,
        >, // maps committee index to committee hash (sha256(x || y)) of aggregate key
        epochs: Map::<u64, EpochProof>, // maps beacon slot to header root and state root
        batches: Map::<felt252, bool>, // Available batch roots
        owner: ContractAddress,
        latest_epoch: u64,
        latest_committee_id: u64,
        initialization_committee: u64,
        committee_update_program_hash: felt252,
        epoch_update_program_hash: felt252,
        epoch_batch_program_hash: felt252,
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
        self.latest_epoch.write(0);

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
            ref self: ContractState, beacon_state_root: u256, committee_hash: u256, slot: u64,
        ) {
            let epoch_proof = self.epochs.read(slot);
            assert(beacon_state_root == epoch_proof.beacon_state_root, 'Invalid State Root!');

            // for now we dont ensure the fact hash is valid
            let fact_hash = compute_committee_proof_fact_hash(
                @self, beacon_state_root, committee_hash, slot,
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
            beacon_state_root: u256,
            slot: u64,
            committee_hash: u256,
            n_signers: u64,
            execution_hash: u256,
            execution_height: u64,
        ) {
            // Add slot validation
            let latest_epoch = self.latest_epoch.read();
            assert(slot > latest_epoch, 'Slot must be higher!');
            
            // println!("verify_epoch_update");
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

            self.latest_epoch.write(slot);
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
            // Add slot validation
            let latest_epoch = self.latest_epoch.read();
            assert(slot > latest_epoch, 'Slot must be higher!');
            
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

            self.latest_epoch.write(slot);
            self.emit(Event::EpochBatch(EpochBatch {
                batch_root: batch_root, beacon_root: header_root, slot: slot, execution_hash: execution_hash, execution_height: execution_height,
            }));

            self.batches.write(batch_root, true);
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

    }

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
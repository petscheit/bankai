/// Interface for the Bankai contract, which manages Ethereum consensus verification on StarkNet
/// This contract enables trustless bridging of Ethereum consensus data to StarkNet
use super::types::EpochProof;

#[starknet::interface]
pub trait IBankaiContract<TContractState> {
    /// Returns the hash of a specific validator committee
    fn get_committee_hash(self: @TContractState, committee_id: u64) -> u256;

    /// Returns the slot number of the most recent verified epoch
    fn get_latest_epoch_slot(self: @TContractState) -> u64;

    /// Returns the ID of the most recent validator committee
    fn get_latest_committee_id(self: @TContractState) -> u64;

    /// Returns the SHARP program hash used for committee updates
    fn get_committee_update_program_hash(self: @TContractState) -> felt252;

    /// Returns the SHARP program hash used for epoch updates
    fn get_epoch_update_program_hash(self: @TContractState) -> felt252;

    /// Returns the SHARP program hash used for epoch batching
    fn get_epoch_batch_program_hash(self: @TContractState) -> felt252;

    /// Retrieves the epoch proof for a given slot
    fn get_epoch_proof(self: @TContractState, slot: u64) -> EpochProof;

    /// Verifies and stores a new validator committee update
    /// @param beacon_state_root - The beacon chain state root containing the committee
    /// @param committee_hash - Hash of the new committee's public key
    /// @param slot - Slot number where this committee becomes active
    fn verify_committee_update(
        ref self: TContractState, beacon_state_root: u256, committee_hash: u256, slot: u64,
    );

    /// Verifies and stores a new epoch update
    /// @param header_root - SSZ root of the beacon block header
    /// @param beacon_state_root - Root of the beacon state
    /// @param slot - Slot number of this epoch
    /// @param committee_hash - Hash of the signing committee
    /// @param n_signers - Number of validators that signed
    /// @param execution_hash - Hash of the execution layer header
    /// @param execution_height - Height of the execution block
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

    /// Verifies and stores a batch of epoch updates
    /// @param batch_root - Merkle root of the batch of epochs
    /// Parameters same as verify_epoch_update
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

    /// Extracts and verifies a single epoch from a previously verified batch
    /// @param batch_root - Root of the verified batch
    /// @param merkle_index - Index of this epoch in the batch
    /// @param merkle_path - Merkle proof path
    /// Other parameters same as verify_epoch_update
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

    /// Proposes an update to the SHARP program hashes (requires owner + timelock)
    fn propose_program_hash_update(
        ref self: TContractState,
        new_committee_hash: felt252,
        new_epoch_hash: felt252,
        new_batch_hash: felt252,
    );

    /// Executes a proposed program hash update after timelock expires
    fn execute_program_hash_update(ref self: TContractState);

    /// Pauses the contract (owner only)
    fn pause(ref self: TContractState);

    /// Unpauses the contract (owner only)
    fn unpause(ref self: TContractState);

    /// Returns whether the contract is currently paused
    fn is_paused(self: @TContractState) -> bool;
}

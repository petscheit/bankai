/// Represents a proof of an Ethereum beacon chain epoch, containing crucial consensus and execution
/// data
#[derive(Drop, starknet::Store, Serde)]
pub struct EpochProof {
    /// Hash of the beacon chain header (SSZ root)
    pub header_root: u256,
    /// State root of the beacon chain at the corresponding slot
    pub beacon_state_root: u256,
    /// Number of validators that signed (out of 512 possible)
    pub n_signers: u64,
    /// Hash of the execution layer (EL) header
    pub execution_hash: u256,
    /// Block height of the execution layer header
    pub execution_height: u64,
}

/// Event emitted when a new committee is validated and stored
#[derive(Drop, starknet::Event)]
pub struct CommitteeUpdated {
    /// Unique identifier for the committee
    pub committee_id: u64,
    /// Aggregate public key hash of the committee
    pub committee_hash: u256,
}

/// Event emitted when a new epoch is validated and stored
#[derive(Drop, starknet::Event)]
pub struct EpochUpdated {
    /// Hash of the beacon header (SSZ root)
    pub beacon_root: u256,
    /// Slot number of the beacon header
    pub slot: u64,
    /// Hash of the execution layer header
    pub execution_hash: u256,
    /// Block height of the execution header
    pub execution_height: u64,
}

/// Event emitted when a batch of epochs is validated
#[derive(Drop, starknet::Event)]
pub struct EpochBatch {
    /// Merkle root of the batch
    pub batch_root: felt252,
    /// Hash of the beacon header
    pub beacon_root: u256,
    /// Slot number
    pub slot: u64,
    /// Hash of the execution header
    pub execution_hash: u256,
    /// Block height of the execution header
    pub execution_height: u64,
}

/// Event emitted when an epoch is extracted from a verified batch
#[derive(Drop, starknet::Event)]
pub struct EpochDecommitted {
    /// Root of the batch containing this epoch
    pub batch_root: felt252,
    /// Slot number
    pub slot: u64,
    /// Hash of the execution header
    pub execution_hash: u256,
    /// Block height of the execution header
    pub execution_height: u64,
}

/// Emitted when the contract is paused
#[derive(Drop, starknet::Event)]
pub struct Paused {}

/// Emitted when the contract is unpaused
#[derive(Drop, starknet::Event)]
pub struct Unpaused {}

mod unpack;

pub struct BeaconHeader {
    pub slot: u64,
    pub proposer_index: u64,
    pub parent_root: u256,
    pub state_root: u256,
    pub body_root: u256,
}
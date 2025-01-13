use crate::constants::{SLOTS_PER_EPOCH, SLOTS_PER_SYNC_COMMITTEE};

pub fn slot_to_epoch_id(slot: u64) -> u64 {
    slot / SLOTS_PER_EPOCH
}

pub fn slot_to_sync_committee_id(slot: u64) -> u64 {
    slot / SLOTS_PER_SYNC_COMMITTEE
}

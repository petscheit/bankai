use ssz_derive::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, Bytes};

#[derive(Debug, Encode, Decode)]
pub struct SyncCommittee {
    pub(crate) pubkeys: Vec<[u8; 48]>,
    pub(crate) aggregate_pubkey: [u8; 48],
}

impl Deserialize for SyncCommittee {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SyncCommittee { pubkeys: Vec::deserialize(deserializer)?, aggregate_pubkey: [0u8; 48] })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LightClientUpdateResponse {
    pub version: String,
    pub data: LightClientUpdate,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct LightClientUpdate {
    pub attested_header: AttestedHeader,
    pub next_sync_committee: SyncCommittee,
    pub next_sync_committee_branch: Vec<String>,
    pub finalized_header: FinalizedHeader,
    pub finality_branch: Vec<String>,
    pub sync_aggregate: SyncAggregate,
    pub signature_slot: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttestedHeader {
    pub beacon: BeaconBlockHeader,
    pub execution: ExecutionHeader,
    pub execution_branch: Vec<String>,
}

pub type FinalizedHeader = AttestedHeader;

#[derive(Debug, Serialize, Deserialize)]
pub struct BeaconBlockHeader {
    pub slot: String,
    pub proposer_index: String,
    pub parent_root: String,
    pub state_root: String,
    pub body_root: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExecutionHeader {
    pub parent_hash: String,
    pub fee_recipient: String,
    pub state_root: String,
    pub receipts_root: String,
    pub logs_bloom: String,
    pub prev_randao: String,
    pub block_number: String,
    pub gas_limit: String,
    pub gas_used: String,
    pub timestamp: String,
    pub extra_data: String,
    pub base_fee_per_gas: String,
    pub block_hash: String,
    pub transactions_root: String,
    pub withdrawals_root: String,
    pub blob_gas_used: String,
    pub excess_blob_gas: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncAggregate {
    pub sync_committee_bits: String,
    pub sync_committee_signature: String,
}

impl Deserialize for SyncCommittee {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SyncCommittee { pubkeys: Vec::deserialize(deserializer)?, aggregate_pubkey: [0u8; 48] })
    }
}
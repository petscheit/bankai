use bls12_381::{G1Affine, G2Affine};
use alloy_rpc_types_beacon::events::light_client_finality::SyncAggregate;
use crate::{types::{EpochProofInputs, SyncCommitteeValidatorPubs}, utils::rpc::BeaconRpcClient, Error};

pub struct EpochUpdate {}

impl EpochUpdate {
    /// Generates a proof for a specific epoch at the given slot
    /// This proof includes the header, signature point, aggregate public key, and non-signing validators
    pub async fn generate_epoch_proof(client: &BeaconRpcClient, mut slot: u64) -> Result<EpochProofInputs, Error> {
        // First attempt with original slot
        let header = match client.get_header(slot).await {
            Ok(header) => header,
            Err(Error::EmptySlotDetected(_)) => {
                slot += 1;
                println!("Empty slot detected! Fetching slot: {}", slot);
                client.get_header(slot).await?
            }
            Err(e) => return Err(e), // Propagate other errors immediately
        };
        
        let sync_agg = client.get_sync_aggregate(slot).await?;
        let validator_pubs = client.get_sync_committee_validator_pubs(slot).await?;
        
        // Process the sync committee data
        let signature_point = Self::extract_signature_point(&sync_agg)?;
        let non_signers = Self::derive_non_signers(&sync_agg, &validator_pubs);
    
        Ok(EpochProofInputs { 
            header: header.into(), 
            signature_point, 
            aggregate_pub: validator_pubs.aggregate_pub, 
            non_signers 
        })
    }
    
    /// Extracts and validates the BLS signature point from the sync aggregate
    fn extract_signature_point(sync_agg: &SyncAggregate) -> Result<G2Affine, Error> {
        let mut bytes = [0u8; 96];
        bytes.copy_from_slice(&sync_agg.sync_committee_signature.0);
        match G2Affine::from_compressed(&bytes).into() {
            Some(point) => Ok(point),
            None => Err(Error::InvalidBLSPoint)
        }
    }
    
    /// Identifies validators who didn't sign the sync committee message
    /// Returns their public keys as G1Affine points
    fn derive_non_signers(sync_aggregate: &SyncAggregate, validator_pubs: &SyncCommitteeValidatorPubs) -> Vec<G1Affine> {
        let bits = Self::convert_bits_to_bool_array(&sync_aggregate.sync_committee_bits);
        validator_pubs.validator_pubs.iter()
            .enumerate()
            .filter_map(|(i, pubkey)| if !bits[i] { Some(*pubkey) } else { None })
            .collect()
    }
    
    /// Converts a byte array of participation bits into a boolean array
    /// Each bit represents whether a validator signed (true) or didn't sign (false)
    fn convert_bits_to_bool_array(bits: &[u8]) -> Vec<bool> {
        bits.iter()
            .flat_map(|byte| (0..8).map(move |i| (byte & (1 << i)) != 0))
            .collect()
    }
}


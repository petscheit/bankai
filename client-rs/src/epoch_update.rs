use bls12_381::{G1Affine, G1Projective, G2Affine};
use alloy_rpc_types_beacon::events::light_client_finality::SyncAggregate;
use crate::{types::{BeaconHeader, EpochProofInputs, SyncCommitteeValidatorPubs}, Error};


pub fn generate_epoch_proof(header: BeaconHeader, sync_aggregate: SyncAggregate, validator_pubs: SyncCommitteeValidatorPubs) -> Result<EpochProofInputs, Error> {
    let mut bytes = [0u8; 96];
    bytes[0..96].copy_from_slice(&sync_aggregate.sync_committee_signature.0);
    let signature_point = G2Affine::from_compressed(&bytes).unwrap();

    let non_signers = derive_non_signers(sync_aggregate, &validator_pubs);

    println!("non_signers: {:#?}", non_signers);
    println!("# of non signers: {}", non_signers.len());

    Ok(EpochProofInputs { header, signature_point, aggregate_pub: validator_pubs.aggregate_pub, non_signers })
}

fn derive_non_signers(sync_aggregate: SyncAggregate, validator_pubs: &SyncCommitteeValidatorPubs) -> Vec<G1Affine> {
    let mut non_signers = Vec::new();
    
    // Convert bytes to bool array
    let bits: Vec<bool> = sync_aggregate.sync_committee_bits
        .iter()
        .flat_map(|byte| {
            (0..8).map(move |i| (byte & (1 << i)) != 0)
        })
        .collect();

    for (i, pubkey) in validator_pubs.validator_pubs.iter().enumerate() {
        if !bits[i] {
            non_signers.push(*pubkey);
        }
    }
    non_signers
}
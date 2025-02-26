use serde::{Deserialize, Serialize};

use crate::Error;

use starknet::core::types::Felt;

use crate::{epoch_update::ExpectedEpochUpdateOutputs, utils::merkle::MerklePath};

#[derive(Debug)]
pub struct BankaiRPCClient {
    endpoint: String,
    api_key: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchMerkleTreeData {
    pub batch_root: Felt,
    pub epoch_index: u64,
    pub path: Vec<MerklePath>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EpochDecommitmentData {
    pub circuit_outputs: ExpectedEpochUpdateOutputs,
    pub merkle_tree: BatchMerkleTreeData,
}

#[derive(Debug, Deserialize)]
pub struct EpochDecommitmentDataResponse {
    pub decommitment_data_for_epoch: EpochDecommitmentData,
    pub epoch_id: u64,
}

impl EpochDecommitmentData {
    pub fn to_calldata(&self) -> Vec<Felt> {
        let (header_root_high, header_root_low) = self
            .circuit_outputs
            .beacon_header_root
            .as_slice()
            .split_at(16);
        let (beacon_state_root_high, beacon_state_root_low) = self
            .circuit_outputs
            .beacon_state_root
            .as_slice()
            .split_at(16);
        let (committee_hash_high, committee_hash_low) =
            self.circuit_outputs.committee_hash.as_slice().split_at(16);
        let (execution_hash_high, execution_hash_low) = self
            .circuit_outputs
            .execution_header_hash
            .as_slice()
            .split_at(16);

        let merkle_path_felts: Vec<Felt> = self
            .merkle_tree
            .path
            .iter()
            .flat_map(|p| {
                //let (value_high, value_low) = p.value.as_slice().split_at(16);
                vec![p.value]
            })
            .collect();

        let mut calldata = vec![
            self.merkle_tree.batch_root,
            Felt::from(self.merkle_tree.epoch_index),
        ];

        // Append merkle path felts
        calldata.extend(merkle_path_felts);

        // Append other fields
        calldata.extend([
            Felt::from_bytes_be_slice(header_root_low),
            Felt::from_bytes_be_slice(header_root_high),
            Felt::from_bytes_be_slice(beacon_state_root_low),
            Felt::from_bytes_be_slice(beacon_state_root_high),
            self.circuit_outputs.slot.into(),
            Felt::from_bytes_be_slice(committee_hash_low),
            Felt::from_bytes_be_slice(committee_hash_high),
            self.circuit_outputs.n_signers.into(),
            Felt::from_bytes_be_slice(execution_hash_low),
            Felt::from_bytes_be_slice(execution_hash_high),
            self.circuit_outputs.execution_header_height.into(),
        ]);

        calldata
    }
}

impl BankaiRPCClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_decommitment_data_for_epoch(
        &self,
        epoch_id: u64,
    ) -> Result<EpochDecommitmentDataResponse, Error> {
        let response = self
            .client
            .get(format!(
                "{}/get_epoch_decommitment_data/by_epoch/{}",
                self.endpoint, epoch_id
            ))
            .header("accept", "application/json")
            .send()
            .await
            .map_err(Error::BankaiRPCClientError)?;

        let response_data: EpochDecommitmentDataResponse =
            response.json().await.map_err(Error::BankaiRPCClientError)?;

        Ok(response_data)
    }
}

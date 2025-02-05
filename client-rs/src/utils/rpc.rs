use crate::constants;
use crate::epoch_update::SyncCommitteeValidatorPubs;
use crate::Error;
use alloy_rpc_types_beacon::events::light_client_finality::SyncAggregate;
use alloy_rpc_types_beacon::header::HeaderResponse;
use itertools::Itertools;
use reqwest::Client;
use serde_json::Value;
use tracing::warn;
use types::eth_spec::MainnetEthSpec;
use types::{BeaconBlockBody, FullPayload};

/// A client for interacting with the Ethereum Beacon Chain RPC endpoints.
/// Provides methods to fetch headers, sync aggregates, and validator information.
#[derive(Debug)]
pub(crate) struct BeaconRpcClient {
    provider: Client,
    pub rpc_url: String,
}

impl BeaconRpcClient {
    /// Creates a new BeaconRpcClient instance.
    ///
    /// # Arguments
    /// * `rpc_url` - The base URL for the Beacon Chain RPC endpoint
    pub fn new(rpc_url: String) -> Self {
        Self {
            provider: reqwest::Client::new(),
            rpc_url,
        }
    }

    /// Makes an HTTP GET request and returns the JSON response.
    /// This is a helper method used by all other RPC calls.
    async fn get_json(&self, route: &str) -> Result<Value, Error> {
        let url = format!("{}/{}", self.rpc_url, route);
        self.provider
            .get(url)
            .send()
            .await
            .map_err(Error::RpcError)?
            .json()
            .await
            .map_err(Error::RpcError)
    }

    // async fn get_ssz_blob(&self, route: &str) -> Result<Bytes, Error> {
    //     let url = format!("{}/{}", self.rpc_url, route);
    //     self.provider
    //         .get(url)
    //         .header("Accept", "application/octet-stream")
    //         .send()
    //         .await
    //         .map_err(Error::RpcError)?
    //         .bytes()
    //         .await
    //         .map_err(Error::RpcError)
    // }

    /// Fetches the beacon chain header for a specific slot.
    /// This provides information about the block at the given slot number.
    /// Returns Error::BlockNotFound if no block exists at the specified slot.
    pub async fn get_header(&self, slot: u64) -> Result<HeaderResponse, Error> {
        let json = self
            .get_json(&format!("eth/v1/beacon/headers/{}", slot))
            .await?;

        // Check for 404 NOT_FOUND error
        if let Some(code) = json.get("code").and_then(|c| c.as_i64()) {
            if code == 404 {
                return Err(Error::EmptySlotDetected(slot));
            }
        }

        serde_json::from_value(json).map_err(|e| Error::DeserializeError(e.to_string()))
    }

    /// Fetches the sync aggregate from the block AFTER the specified slot.
    /// Note: This intentionally fetches slot + 1 because sync aggregates reference
    /// the previous slot's header.
    pub async fn get_sync_aggregate(&self, mut slot: u64) -> Result<SyncAggregate, Error> {
        slot += 1; // signature is in the next slot

        let mut attempts = 0;

        // Ensure the slot is not missed and increment in case it is
        let _header = loop {
            match self.get_header(slot).await {
                Ok(header) => break header,
                Err(Error::EmptySlotDetected(_)) => {
                    attempts += 1;
                    if attempts >= constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS {
                        return Err(Error::EmptySlotDetected(slot));
                    }
                    slot += 1;
                    warn!(
                        "Empty slot detected! Attempt {}/{}. Fetching slot: {}",
                        attempts,
                        constants::MAX_SKIPPED_SLOTS_RETRY_ATTEMPTS,
                        slot
                    );
                }
                Err(e) => return Err(e), // Propagate other errors immediately
            }
        };

        let json = self
            .get_json(&format!("eth/v2/beacon/blocks/{}", slot))
            .await?;

        serde_json::from_value(json["data"]["message"]["body"]["sync_aggregate"].clone())
            .map_err(|e| Error::DeserializeError(e.to_string()))
    }

    /// Retrieves the list of validator indices that are part of the sync committee
    /// for the specified slot.
    ///
    /// The returned indices can be used to fetch the corresponding public keys
    /// using fetch_validator_pubkeys().
    async fn fetch_sync_committee_indexes(&self, slot: u64) -> Result<Vec<u64>, Error> {
        let json = self
            .get_json(&format!("eth/v1/beacon/states/{}/sync_committees", slot))
            .await?;

        // Parse the array of validator indices from the JSON response
        json["data"]["validators"]
            .as_array()
            .ok_or(Error::FetchSyncCommitteeError)?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or(Error::FetchSyncCommitteeError)
                    .and_then(|s| s.parse().map_err(|_| Error::FetchSyncCommitteeError))
            })
            .collect()
    }

    /// Fetches the public keys for a list of validator indices.
    ///
    /// # Arguments
    /// * `indexes` - Array of validator indices to look up
    ///
    /// # Returns
    /// A vector of public keys in the same order as the input indices.
    /// If a validator index is not found, returns an error.
    async fn fetch_validator_pubkeys(&self, indexes: &[u64]) -> Result<Vec<String>, Error> {
        // Construct query string with all validator indices
        let query = indexes.iter().map(|i| format!("id={}", i)).join("&");
        let json = self
            .get_json(&format!("eth/v1/beacon/states/head/validators?{}", query))
            .await?;

        let validators = json["data"]
            .as_array()
            .ok_or(Error::FetchSyncCommitteeError)?;

        // Match returned validators with requested indices and extract public keys
        indexes
            .iter()
            .map(|index| {
                validators
                    .iter()
                    .find(|v| {
                        v["index"].as_str().and_then(|i| i.parse::<u64>().ok()) == Some(*index)
                    })
                    .and_then(|v| v["validator"]["pubkey"].as_str())
                    .map(String::from)
                    .ok_or(Error::FetchSyncCommitteeError)
            })
            .collect()
    }

    pub async fn get_block_body(
        &self,
        slot: u64,
    ) -> Result<BeaconBlockBody<MainnetEthSpec, FullPayload<MainnetEthSpec>>, Error> {
        let json = self
            .get_json(&format!("eth/v2/beacon/blocks/{}", slot))
            .await?;

        let block: BeaconBlockBody<MainnetEthSpec, FullPayload<MainnetEthSpec>> =
            serde_json::from_value(json["data"]["message"]["body"].clone()).unwrap();

        Ok(block)
    }

    /// Fetches the public keys of validators in the sync committee for a given slot.
    /// Note: This actually fetches data for the next slot (slot + 1).
    ///
    /// # Arguments
    /// * `slot` - The slot number to fetch the sync committee validator public keys for
    ///
    /// # Returns
    /// Returns a `SyncCommitteeValidatorPubs` containing the public keys of all validators
    /// in the sync committee.
    pub async fn get_sync_committee_validator_pubs(
        &self,
        slot: u64,
    ) -> Result<SyncCommitteeValidatorPubs, Error> {
        let slot = slot + 1;
        let indexes = self.fetch_sync_committee_indexes(slot).await?;
        let pubkeys = self.fetch_validator_pubkeys(&indexes).await?;
        Ok(pubkeys.into())
    }

    /// Fetches the current head slot of the beacon chain.
    ///
    /// # Returns
    /// The current slot number of the beacon chain head.
    pub async fn get_head_slot(&self) -> Result<u64, Error> {
        let json = self.get_json("eth/v1/beacon/headers/head").await?;

        let slot = json["data"]["header"]["message"]["slot"]
            .as_str()
            .ok_or(Error::DeserializeError("Missing slot field".into()))?
            .parse()
            .map_err(|e: std::num::ParseIntError| Error::DeserializeError(e.to_string()))?;

        Ok(slot)
    }
}

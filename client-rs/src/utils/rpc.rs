use reqwest::{Client, Response};
use alloy_rpc_types_beacon::header::HeaderResponse;
use alloy_rpc_types_beacon::events::light_client_finality::SyncAggregate;
use serde_json::Value;
use crate::Error;
use itertools::Itertools;
use crate::types::SyncCommitteeValidatorPubs;


pub(crate) struct BeaconRpcClient {
    provider: Client,
    pub rpc_url: String,
}

impl BeaconRpcClient {
    pub fn new(rpc_url: String) -> Self {
        Self { provider: reqwest::Client::new(), rpc_url }
    }

    async fn make_request(&self, route: &str) -> Result<Response, Error> {
        let url = format!("{}/{}", self.rpc_url, route);
        let res = self.provider.get(url).send().await.map_err(Error::RpcError)?;
        Ok(res)
    }

    pub async fn get_header(&self, slot: u64) -> Result<HeaderResponse, Error> {
        let resp: HeaderResponse = self.make_request(&format!("eth/v1/beacon/headers/{}", slot))
            .await?
            .json::<HeaderResponse>()
            .await.map_err(Error::RpcError)?;
        Ok(resp)
    }

    pub async fn get_sync_aggregate(&self, slot: u64) -> Result<SyncAggregate, Error> {
        let resp = self.make_request(&format!("eth/v2/beacon/blocks/{}", slot + 1))
            .await?
            .json::<Value>()
            .await.map_err(Error::RpcError)?;


        let sync_agg: SyncAggregate = serde_json::from_value(resp["data"]["message"]["body"]["sync_aggregate"].clone()).map_err(Error::DeserializeError)?;
        Ok(sync_agg)
    }

    async fn fetch_sync_committee_indexes(&self, slot: u64) -> Result<Vec<u64>, Error> {
        let endpoint = format!("eth/v1/beacon/states/{}/sync_committees", slot);
        let response = self.make_request(&endpoint)
            .await?
            .json::<Value>()
            .await
            .map_err(Error::RpcError)?;
        
        response["data"].get("validators")
            .ok_or_else(|| Error::FetchSyncCommitteeError)?
            .as_array()
            .ok_or_else(|| Error::FetchSyncCommitteeError)?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| Error::FetchSyncCommitteeError)
                    .and_then(|id_str| id_str.parse::<u64>().map_err(|_| Error::FetchSyncCommitteeError))
            })
            .collect()
    }

    async fn fetch_validator_pubkeys(&self, indexes: &[u64]) -> Result<Vec<String>, Error> {
        let query = indexes.iter()
            .map(|i| format!("id={}", i))
            .join("&");
        
        let validators: Value = self.make_request(&format!("eth/v1/beacon/states/head/validators?{}", query))
            .await?
            .json()
            .await
            .map_err(Error::RpcError)?;
        
        let validators_data = validators["data"].as_array()
            .ok_or_else(|| Error::FetchSyncCommitteeError)?;

        indexes.iter()
            .map(|index| {
                validators_data
                    .iter()
                    .find(|v| v["index"].as_str().map(|i| i.parse::<u64>().unwrap_or(u64::MAX)) == Some(*index))
                    .ok_or_else(|| Error::FetchSyncCommitteeError)
                    .and_then(|entry| {
                        entry["validator"]["pubkey"]
                            .as_str()
                            .ok_or_else(|| Error::FetchSyncCommitteeError)
                            .map(String::from)
                    })
            })
            .collect()
    }

    pub async fn get_sync_committee_validator_pubs(&self, slot: u64) -> Result<SyncCommitteeValidatorPubs, Error> {
        let slot = slot + 1;
        let indexes = self.fetch_sync_committee_indexes(slot).await?;
        let pubkeys = self.fetch_validator_pubkeys(&indexes).await?;
        Ok(pubkeys.into())
    }
}

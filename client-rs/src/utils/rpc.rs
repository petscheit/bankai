use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct BeaconResponse {
    execution_optimistic: bool,
    finalized: bool,
    data: BeaconData,
}

#[derive(Debug, Serialize, Deserialize)]
struct BeaconData {
    root: String,
}

pub async fn get_state_root(slot: u64, beacon_node_url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let url = format!("{}/eth/v1/beacon/states/{}/root", beacon_node_url, slot);

    let response = client
        .get(&url)
        .send()
        .await?
        .json::<BeaconResponse>()
        .await?;

    Ok(response.data.root)
}

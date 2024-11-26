mod types;
mod rpc;

#[tokio::main]
async fn main() {
    let rpc_client = rpc::RpcClient::new("http://testing.mainnet.beacon-api.nimbus.team".to_string());
    
    match rpc_client.fetch_light_client_update(1267).await {
        Ok(_) => println!("Successfully fetched light client update"),
        Err(e) => eprintln!("Error fetching light client update: {:?}", e),
    }
}

#[derive(Debug)]
pub enum Error {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    LightClientUpdatePeriodNotAvailable(u32)
}

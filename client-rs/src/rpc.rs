use reqwest::Client;
use crate::types::LightClientUpdateResponse;
use crate::Error;
pub(crate) struct RpcClient {
    client: Client,
    base_url: String,
}

impl RpcClient {
    pub fn new(base_url: String) -> Self {
        Self { client: Client::new(), base_url }
    }

    pub async fn fetch_light_client_update(&self, start_period: u64) -> Result<(), Error> {
        let url = format!(
            "{}/eth/v1/beacon/light_client/updates?start_period={}&count=2", 
            self.base_url, 
            start_period
        );
        
        let response = self.client.get(url).send().await.map_err(Error::Reqwest)?;

        let response_body: serde_json::Value = response.json().await.map_err(Error::Reqwest)?;
        // println!("{:#?}", response_body);
        let update: Vec<LightClientUpdateResponse> = serde_json::from_value(response_body).map_err(Error::Serde)?;
        println!("{:#?}", update);

       
        Ok(())
    }   
}

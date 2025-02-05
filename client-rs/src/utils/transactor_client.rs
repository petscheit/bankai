use crate::{config::BankaiConfig, traits::Submittable, Error};
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, trace};

#[derive(Debug)]
pub struct TransactorClient {
    endpoint: String,
    api_key: String,
    pub client: Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactorResponse {
    pub transactor_status: String,
    pub tx: TransactionDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub hash: Option<String>,
    pub multicall_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TransactorRequest {
    pub chain_id: String,
    pub contract_invocations: Vec<ContractInvocation>,
}

#[derive(Debug, Serialize)]
pub struct ContractInvocation {
    pub value: String,
    pub chain_id: String,
    pub calldata: String,
    pub method_selector: String,
    pub contract_address: String,
}

impl TransactorClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: Client::new(),
        }
    }

    pub async fn send_transaction(
        &self,
        request: TransactorRequest,
    ) -> Result<TransactorResponse, Error> {
        let url = format!("{}/transactor", self.endpoint);
        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await
            .map_err(Error::TransactorError)?;

        let response_data: TransactorResponse =
            response.json().await.map_err(Error::TransactorError)?;
        Ok(response_data)
    }

    pub async fn check_transaction_status(
        &self,
        transaction_id: &str,
    ) -> Result<TransactorResponse, Error> {
        let url = format!("{}/transactor/{}", self.endpoint, transaction_id);
        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await
            .map_err(Error::TransactorError)?;

        let response_data: TransactorResponse =
            response.json().await.map_err(Error::TransactorError)?;
        Ok(response_data)
    }

    pub async fn poll_transaction_status_until_done(
        &self,
        transaction_id: &str,
        sleep_duration: Duration,
        max_retries: usize,
    ) -> Result<bool, Error> {
        for attempt in 1..=max_retries {
            debug!("Polling Transactor for update... {}", transaction_id);
            let status_response = self.check_transaction_status(transaction_id).await?;
            let status = status_response.transactor_status;

            if status == "OK_SUCCESS" {
                return Ok(true);
            }

            if status == "KO_FAILED_TO_ESTIMATE_GAS" || status == "KO_WITH_ERRORS" {
                return Err(Error::InvalidResponse(format!(
                    "Transactor processing failed for transaction {} with status: {}",
                    transaction_id, status
                )));
            }

            trace!(
                "Transaction {} not completed yet. Status: {}. Polling attempt {}/{}",
                transaction_id,
                status,
                attempt,
                max_retries
            );
            sleep(sleep_duration).await;
        }

        Err(Error::InvalidResponse(format!(
            "Polling timeout for transaction {}",
            transaction_id
        )))
    }

    pub async fn submit_update<T>(
        &self,
        update: impl Submittable<T>,
        config: &BankaiConfig,
    ) -> Result<String, Error> {
        let request = TransactorRequest {
            chain_id: config.proof_settlement_chain_id.clone().to_hex_string(),
            contract_invocations: vec![ContractInvocation {
                value: "0".to_string(),
                chain_id: config.proof_settlement_chain_id.clone().to_hex_string(),
                calldata: update
                    .to_calldata()
                    .iter()
                    .map(|felt| felt.to_hex_string())
                    .collect(),
                method_selector: "".to_string(),
                contract_address: config.contract_address.clone().to_hex_string(),
            }],
        };

        let response = self.send_transaction(request).await?;

        if let Some(hash) = response.tx.hash {
            println!("Transaction sent with tx_hash: {:?}", hash);
            Ok(hash)
        } else {
            Err(Error::InvalidResponse(
                "Transaction hash not found".to_string(),
            ))
        }
    }
}

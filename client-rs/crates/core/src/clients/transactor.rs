//! Transactor Client Implementation
//! 
//! This module provides a client for interacting with a transaction processing service.
//! It handles transaction submission, status checking, and polling for transaction completion.
//! The transactor service provides an abstraction layer for submitting and managing blockchain transactions.

use crate::{types::traits::Submittable, utils::config::BankaiConfig};
use reqwest::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tracing::{debug, trace};

/// Client for interacting with the transactor service
/// 
/// Provides functionality to submit transactions and monitor their status
/// through a REST API endpoint with authentication.
#[derive(Debug)]
pub struct TransactorClient {
    /// Base URL of the transactor service
    endpoint: String,
    /// API key for authentication
    api_key: String,
    /// HTTP client for making requests
    pub client: Client,
}

/// Response from the transactor service containing transaction status and details
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactorResponse {
    /// Current status of the transaction in the transactor service
    pub transactor_status: String,
    /// Detailed information about the transaction
    pub tx: TransactionDetails,
}

/// Detailed information about a transaction
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetails {
    /// Transaction hash if available
    pub hash: Option<String>,
    /// Status of the multicall operation if applicable
    pub multicall_status: Option<String>,
}

/// Request structure for submitting transactions to the transactor
#[derive(Debug, Serialize)]
pub struct TransactorRequest {
    /// Chain identifier for the target blockchain
    pub chain_id: String,
    /// List of contract invocations to be executed
    pub contract_invocations: Vec<ContractInvocation>,
}

/// Structure representing a single contract invocation
#[derive(Debug, Serialize)]
pub struct ContractInvocation {
    /// Value to be sent with the transaction
    pub value: String,
    /// Chain identifier for this specific invocation
    pub chain_id: String,
    /// Encoded calldata for the contract interaction
    pub calldata: String,
    /// Method selector/function signature
    pub method_selector: String,
    /// Target contract address
    pub contract_address: String,
}

impl TransactorClient {
    /// Creates a new transactor client instance
    ///
    /// # Arguments
    /// * `endpoint` - Base URL of the transactor service
    /// * `api_key` - API key for authentication
    ///
    /// # Returns
    /// * A new TransactorClient instance
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: Client::new(),
        }
    }

    /// Sends a transaction to the transactor service
    ///
    /// # Arguments
    /// * `request` - Transaction request containing chain ID and contract invocations
    ///
    /// # Returns
    /// * `Result<TransactorResponse, TransactorError>` - Response from the transactor or error
    pub async fn send_transaction(
        &self,
        request: TransactorRequest,
    ) -> Result<TransactorResponse, TransactorError> {
        let url = format!("{}/transactor", self.endpoint);
        let response = self
            .client
            .post(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .json(&request)
            .send()
            .await?;

        let response_data: TransactorResponse = response.json().await?;
        Ok(response_data)
    }

    /// Checks the current status of a transaction
    ///
    /// # Arguments
    /// * `transaction_id` - ID of the transaction to check
    ///
    /// # Returns
    /// * `Result<TransactorResponse, TransactorError>` - Current transaction status or error
    pub async fn check_transaction_status(
        &self,
        transaction_id: &str,
    ) -> Result<TransactorResponse, TransactorError> {
        let url = format!("{}/transactor/{}", self.endpoint, transaction_id);
        let response = self
            .client
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.api_key))
            .header(CONTENT_TYPE, "application/json")
            .send()
            .await?;

        let response_data: TransactorResponse = response.json().await?;
        Ok(response_data)
    }

    /// Polls the transaction status until completion or timeout
    ///
    /// # Arguments
    /// * `transaction_id` - ID of the transaction to monitor
    /// * `sleep_duration` - Duration to wait between polling attempts
    /// * `max_retries` - Maximum number of polling attempts
    ///
    /// # Returns
    /// * `Result<bool, TransactorError>` - Success status or error
    pub async fn poll_transaction_status_until_done(
        &self,
        transaction_id: &str,
        sleep_duration: Duration,
        max_retries: usize,
    ) -> Result<bool, TransactorError> {
        for attempt in 1..=max_retries {
            debug!("Polling Transactor for update... {}", transaction_id);
            let status_response = self.check_transaction_status(transaction_id).await?;
            let status = status_response.transactor_status;

            if status == "OK_SUCCESS" {
                return Ok(true);
            }

            if status == "KO_FAILED_TO_ESTIMATE_GAS" || status == "KO_WITH_ERRORS" {
                return Err(TransactorError::Response(format!(
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

        Err(TransactorError::PollingTimeout(format!(
            "Polling timeout for transaction {}",
            transaction_id
        )))
    }

    /// Submits an update through the transactor service
    ///
    /// # Arguments
    /// * `update` - Update data implementing the Submittable trait
    /// * `config` - Configuration containing contract and chain details
    ///
    /// # Returns
    /// * `Result<String, TransactorError>` - Transaction hash or error
    pub async fn submit_update<T>(
        &self,
        update: impl Submittable<T>,
        config: &BankaiConfig,
    ) -> Result<String, TransactorError> {
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
            Err(TransactorError::Response(
                "Transaction hash not found".to_string(),
            ))
        }
    }
}

/// Possible errors that can occur during transactor operations
#[derive(Debug, Error)]
pub enum TransactorError {
    /// Error occurring during HTTP request
    #[error("Transactor request error: {0}")]
    Request(#[from] reqwest::Error),
    /// Error in transactor service response
    #[error("Transactor response error: {0}")]
    Response(String),
    /// Error when polling exceeds maximum retries
    #[error("Polling timeout for transaction {0}")]
    PollingTimeout(String),
}

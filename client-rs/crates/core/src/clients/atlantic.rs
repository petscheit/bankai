use std::{env, path::PathBuf};

use futures::StreamExt;
use reqwest::{
    multipart::{Form, Part},
    Body,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    fs,
    time::{sleep, Duration},
};
use tokio_util::io::ReaderStream;
use tracing::{debug, error, info, trace};

use crate::{
    types::traits::{ProofType, Provable},
};

#[derive(Debug)]
pub struct AtlanticClient {
    endpoint: String,
    api_key: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarkProof {
    pub proof: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum AtlanticError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Atlantic error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Atlantic processing error: {0}")]
    AtlanticProcessingError(String),
    #[error("Pooling timeout for batch: {0}")]
    AtlanticPoolingTimeout(String),
    #[error("Decoding Error: {0}")]
    Decoding(#[from] serde_json::Error),
}

impl AtlanticClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn submit_batch(&self, batch: impl Provable) -> Result<String, AtlanticError> {
        let pie_path: PathBuf = batch.pie_path().into();

        let meta = fs::metadata(pie_path.clone()).await?;
        let total_bytes = meta.len();

        let file = fs::File::open(pie_path.clone()).await?;

        let stream = ReaderStream::new(file);

        let progress_stream = stream.scan(
            (0_u64, 10_u64),
            move |(uploaded, next_threshold), chunk_result| {
                match chunk_result {
                    Ok(chunk) => {
                        *uploaded += chunk.len() as u64;
                        let percent = (*uploaded as f64 / total_bytes as f64) * 100.0;

                        if percent >= *next_threshold as f64 && *next_threshold <= 100 {
                            info!(
                                "Uploaded {}% of the PIE file to Atlantic API...",
                                *next_threshold
                            );
                            *next_threshold += 10;
                        }

                        // Pass the chunk further down the stream
                        futures::future::ready(Some(Ok(chunk)))
                    }
                    Err(e) => {
                        // Forward the error
                        futures::future::ready(Some(Err(e)))
                    }
                }
            },
        );

        // Read the file as bytes
        // let file_bytes = fs::read(&pie_path).map_err(Error::IoError)?;
        // let file_part = Part::bytes(file_bytes)
        //     .file_name(pie_path) // Provide a filename
        //     .mime_str("application/zip") // Specify MIME type
        //     .map_err(Error::AtlanticError)?;
        let file_part = Part::stream(Body::wrap_stream(progress_stream))
            .file_name(
                pie_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            )
            .mime_str("application/zip")?;

        let external_id = format!(
            "update_{}",
            match batch.proof_type() {
                ProofType::Epoch => "epoch",
                ProofType::SyncCommittee => "sync_committee",
                ProofType::EpochBatch => "epoch_batch",
            }
        );
        // Build the form
        let form = Form::new()
            .part("pieFile", file_part)
            .text("layout", "auto")
            .text("prover", "starkware_sharp")
            .text("externalId", external_id);

        // Send the request
        let response = self
            .client
            .post(format!("{}/v1/proof-generation", self.endpoint))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AtlanticError::InvalidResponse(format!(
                "Request failed: {}",
                error_text
            )));
        }

        // Parse the response
        let response_data: serde_json::Value = response.json().await?;

        Ok(response_data["atlanticQueryId"]
            .as_str()
            .ok_or_else(|| AtlanticError::InvalidResponse("Missing atlanticQueryId".into()))?
            .to_string())
    }

    pub async fn submit_wrapped_proof(&self, proof: StarkProof) -> Result<String, AtlanticError> {
        info!("Uploading to Atlantic...");
        // Serialize the proof to JSON string
        let proof_json = serde_json::to_string(&proof)?;

        // Create a Part from the JSON string
        let proof_part = Part::text(proof_json)
            .file_name("proof.json")
            .mime_str("application/json")?;

        // Build the form
        let form = Form::new()
            .text(
                "programHash",
                env::var("PROOF_WRAPPER_PROGRAM_HASH").unwrap(),
            )
            .part("inputFile", proof_part)
            .text("cairoVersion", "0")
            .text("mockFactHash", "false")
            .text("externalId", "proof_wrapper");

        // Send the request
        let response = self
            .client
            .post(format!("{}/v1/l2/atlantic-query", self.endpoint))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(AtlanticError::InvalidResponse(format!(
                "Request failed: {}",
                error_text
            )));
        }

        // Parse the response
        let response_data: serde_json::Value = response.json().await?;

        Ok(response_data["atlanticQueryId"]
            .as_str()
            .ok_or_else(|| AtlanticError::InvalidResponse("Missing atlanticQueryId".into()))?
            .to_string())
    }

    pub async fn fetch_proof(&self, batch_id: &str) -> Result<StarkProof, AtlanticError> {
        let response = self
            .client
            .get(format!(
                "{}/query_{}/proof.json",
                env::var("PROOF_REGISTRY").unwrap(),
                batch_id
            ))
            .header("accept", "application/json")
            .send()
            .await?;

        let response_data: serde_json::Value =
            response.json().await?;

        Ok(StarkProof {
            proof: response_data,
        })
    }

    pub async fn check_batch_status(&self, batch_id: &str) -> Result<String, AtlanticError> {
        let response = self
            .client
            .get(format!("{}/v1/atlantic-query/{}", self.endpoint, batch_id))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .send()
            .await?;

        let response_data: serde_json::Value = response.json().await?;

        let status = response_data["atlanticQuery"]["status"]
            .as_str()
            .ok_or_else(|| AtlanticError::InvalidResponse("Missing status field".into()))?;

        Ok(status.to_string())
    }

    pub async fn poll_batch_status_until_done(
        &self,
        batch_id: &str,
        sleep_duration: Duration,
        max_retries: usize,
    ) -> Result<bool, AtlanticError> {
        for attempt in 1..=max_retries {
            debug!("Pooling Atlantic for update... {}", batch_id);
            let status = self.check_batch_status(batch_id).await?;

            if status == "DONE" {
                return Ok(true);
            }

            if status == "FAILED" {
                return Err(AtlanticError::AtlanticProcessingError(format!(
                    "Atlantic processing failed for query {}",
                    batch_id
                )));
            }

            trace!(
                "Batch {} not completed yet. Status: {}. Pooling attempt {}/{}",
                batch_id,
                status,
                attempt,
                max_retries
            );
            sleep(sleep_duration).await;
        }

        return Err(AtlanticError::AtlanticPoolingTimeout(format!(
            "Pooling timeout for batch {}",
            batch_id
        )));
    }
}

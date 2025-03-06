//! Atlantic Client Module
//! 
//! This module provides functionality to interact with the Atlantic API for proof generation
//! and verification. It handles file uploads, proof submissions, and status polling for
//! batch processing operations.

use std::{env, path::PathBuf};

use cairo_vm::vm::runners::cairo_pie::CairoPie;
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

use crate::types::traits::{ProofType, Provable};

/// Client for interacting with the Atlantic API service.
/// 
/// Provides methods for submitting proofs, checking batch statuses,
/// and retrieving generated proofs from the Atlantic service.
#[derive(Debug)]
pub struct AtlanticClient {
    endpoint: String,
    api_key: String,
    pub client: reqwest::Client,
}

/// Represents a STARK proof structure returned by the Atlantic service.
#[derive(Debug, Serialize, Deserialize)]
pub struct StarkProof {
    pub proof: serde_json::Value,
}

/// Possible errors that can occur during Atlantic API operations.
#[derive(Debug, Error)]
pub enum AtlanticError {
    /// IO-related errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// HTTP request errors
    #[error("Atlantic error: {0}")]
    Request(#[from] reqwest::Error),
    /// Invalid API response errors
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    /// Errors during proof processing
    #[error("Atlantic processing error: {0}")]
    AtlanticProcessingError(String),
    /// Timeout errors during batch processing
    #[error("Pooling timeout for batch: {0}")]
    AtlanticPoolingTimeout(String),
    /// JSON decoding errors
    #[error("Decoding Error: {0}")]
    Decoding(#[from] serde_json::Error),
}

impl AtlanticClient {
    /// Creates a new Atlantic client instance.
    /// 
    /// # Arguments
    /// * `endpoint` - The base URL for the Atlantic API
    /// * `api_key` - Authentication key for API access
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Submits a batch for proof generation.
    /// 
    /// Uploads a PIE file to the Atlantic API and initiates proof generation.
    /// Displays progress during file upload.
    /// 
    /// # Arguments
    /// * `pie` - The generate Pie
    /// 
    /// # Returns
    /// * `Result<String, AtlanticError>` - The Atlantic query ID on success
    pub async fn submit_batch(&self, pie: CairoPie, proof_type: ProofType) -> Result<String, AtlanticError> {
        let pie_path = std::env::temp_dir().join("pie.zip");
        pie.write_zip_file(&pie_path, true)?;
        println!("{}", pie_path.display());
        let file = fs::File::open(pie_path.clone()).await?;
        
        // Get file metadata to determine total size
        let metadata = fs::metadata(&pie_path).await?;
        let total_bytes = metadata.len();

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
            match proof_type {
                ProofType::Epoch => "epoch",
                ProofType::SyncCommittee => "sync_committee",
                ProofType::EpochBatch => "epoch_batch",
            }
        );
        
        // Build the form with updated API parameters
        let form = Form::new()
            .part("pieFile", file_part)
            .text("declaredJobSize", "S")
            .text("layout", "dynamic")
            .text("cairoVm", "rust")
            .text("cairoVersion", "cairo0")
            .text("result", "PROOF_GENERATION")
            .text("externalId", external_id);

        // Send the request to the updated endpoint
        let response = self
            .client
            .post(format!("{}/atlantic-query", self.endpoint))
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

    /// Submits a wrapped proof to the Atlantic API.
    /// 
    /// # Arguments
    /// * `proof` - The STARK proof to be wrapped
    /// 
    /// # Returns
    /// * `Result<String, AtlanticError>` - The Atlantic query ID on success
    pub async fn submit_wrapped_proof(&self, proof: StarkProof, program_path: String) -> Result<String, AtlanticError> {
        info!("Uploading to Atlantic...");
        // Serialize the proof to JSON string
        let proof_json = serde_json::to_string(&proof)?;
        
        let program = fs::read(program_path).await?;
        let program_part = Part::bytes(program)
            .file_name("program.json") // Provide a filename
            .mime_str("application/json")?;

        // Create a Part from the JSON string
        let proof_part = Part::text(proof_json)
            .file_name("proof.json")
            .mime_str("application/json")?;

        // Build the form with updated API parameters
        let form = Form::new()
            .part("programFile", program_part)
            .part("inputFile", proof_part)
            .text("declaredJobSize", "M")
            .text("cairoVersion", "cairo0")
            .text("cairoVm", "python")
            .text("layout", "recursive_with_poseidon")
            .text("result", "PROOF_VERIFICATION_ON_L2")
            .text("mockFactHash", "false")
            .text("externalId", "proof_wrapper");

        // Send the request to the updated endpoint
        let response = self
            .client
            .post(format!("{}/atlantic-query", self.endpoint))
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

    /// Fetches a generated proof from the proof registry.
    /// 
    /// # Arguments
    /// * `batch_id` - The ID of the batch to fetch the proof for
    /// 
    /// # Returns
    /// * `Result<StarkProof, AtlanticError>` - The generated STARK proof
    pub async fn fetch_proof(&self, batch_id: &str) -> Result<StarkProof, AtlanticError> {
        let response = self
            .client
            .get(format!(
                "{}/{}/proof.json",
                env::var("PROOF_REGISTRY").unwrap(),
                batch_id
            ))
            .header("accept", "application/json")
            .send()
            .await?;

        let response_data: serde_json::Value = response.json().await?;

        Ok(StarkProof {
            proof: response_data,
        })
    }

    /// Checks the current status of a batch processing request.
    /// 
    /// # Arguments
    /// * `batch_id` - The ID of the batch to check
    /// 
    /// # Returns
    /// * `Result<String, AtlanticError>` - The current status of the batch
    pub async fn check_batch_status(&self, batch_id: &str) -> Result<String, AtlanticError> {
        let response = self
            .client
            .get(format!("{}/atlantic-query/{}", self.endpoint, batch_id))
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

    /// Polls the batch status until completion or failure.
    /// 
    /// # Arguments
    /// * `batch_id` - The ID of the batch to poll
    /// * `sleep_duration` - Duration to wait between polling attempts
    /// * `max_retries` - Maximum number of polling attempts
    /// 
    /// # Returns
    /// * `Result<bool, AtlanticError>` - True if batch completed successfully
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

        Err(AtlanticError::AtlanticPoolingTimeout(format!(
            "Pooling timeout for batch {}",
            batch_id
        )))
    }
}

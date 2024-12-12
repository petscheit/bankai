use std::{env, fs};

use crate::traits::Provable;
use crate::Error;
use reqwest::multipart::{Form, Part};
use serde::{Serialize, Deserialize};
pub struct AtlanticClient {
    endpoint: String,
    api_key: String,
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarkProof {
    pub proof: serde_json::Value,
}

impl AtlanticClient {
    pub fn new(endpoint: String, api_key: String) -> Self {
        Self {
            endpoint,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    pub async fn submit_batch(&self, batch: impl Provable) -> Result<String, Error> {
        let pie_path = batch.pie_path();

        // Read the file as bytes
        let file_bytes = fs::read(&pie_path).map_err(|e| Error::IoError(e))?;
        let file_part = Part::bytes(file_bytes)
            .file_name(pie_path) // Provide a filename
            .mime_str("application/zip") // Specify MIME type
            .map_err(|e| Error::AtlanticError(e))?;

        // Build the form
        let form = Form::new()
            .part("pieFile", file_part)
            .text("layout", "auto")
            .text("prover", "starkware_sharp")
            .text("externalId", "");

        // Send the request
        let response = self
            .client
            .post(format!("{}/v1/proof-generation", self.endpoint))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::AtlanticError(e))?;

        if !response.status().is_success() {
            println!("Error status: {}", response.status());
            let error_text = response.text().await.map_err(|e| Error::AtlanticError(e))?;
            println!("Error response: {}", error_text);
            return Err(Error::InvalidResponse(format!("Request failed: {}", error_text)));
        }

        // Parse the response
        let response_data: serde_json::Value =
            response.json().await.map_err(|e| Error::AtlanticError(e))?;

            
        Ok(response_data["atlanticQueryId"]
            .as_str()
            .ok_or_else(|| Error::InvalidResponse("Missing atlanticQueryId".into()))?
            .to_string())
    }

    pub async fn submit_wrapped_proof(&self, proof: StarkProof) -> Result<String, Error> {
        println!("Uploading to Atlantic!");
        // Serialize the proof to JSON string
        let proof_json = serde_json::to_string(&proof)
            .map_err(|e| Error::DeserializeError(e.to_string()))?;

        // Create a Part from the JSON string
        let proof_part = Part::text(proof_json)
            .file_name("proof.json")
            .mime_str("application/json")
            .map_err(|e| Error::AtlanticError(e))?;
        
        let verifier = fs::read("cairo-verifier/program.json").map_err(|e| Error::IoError(e))?;
        let verifier_part = Part::bytes(verifier)
            .file_name("program.json") // Provide a filename
            .mime_str("application/json") // Specify MIME type
            .map_err(|e| Error::AtlanticError(e))?;

        // Build the form
        let form = Form::new()
            .part("programFile", verifier_part)
            .part("inputFile", proof_part)
            .text("cairoVersion", "0")
            .text("mockFactHash", "false");

        // Send the request
        let response = self
            .client
            .post(format!("{}/v1/l2/atlantic-query", self.endpoint))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::AtlanticError(e))?;

        if !response.status().is_success() {
            let error_text = response.text().await.map_err(|e| Error::AtlanticError(e))?;
            return Err(Error::InvalidResponse(format!("Request failed: {}", error_text)));
        }

        // Parse the response
        let response_data: serde_json::Value =
            response.json().await.map_err(|e| Error::AtlanticError(e))?;
            
        Ok(response_data["atlanticQueryId"]
            .as_str()
            .ok_or_else(|| Error::InvalidResponse("Missing atlanticQueryId".into()))?
            .to_string())
    }

    pub async fn fetch_proof(&self, batch_id: &str) -> Result<StarkProof, Error> {
        let response = self
            .client
            .get(format!(
                "{}/query_{}/proof.json",
                env::var("PROOF_REGISTRY").unwrap(),
                batch_id
            ))
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::AtlanticError(e))?;

        let response_data: serde_json::Value =
            response.json().await.map_err(|e| Error::AtlanticError(e))?;

        Ok(StarkProof {
            proof: response_data,
        })
    }

    pub async fn check_batch_status(&self, batch_id: &str) -> Result<String, Error> {
        let response = self
            .client
            .get(format!("{}/v1/atlantic-query/{}", self.endpoint, batch_id))
            .query(&[("apiKey", &self.api_key)])
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| Error::AtlanticError(e))?;

        let response_data: serde_json::Value = 
            response.json().await.map_err(|e| Error::AtlanticError(e))?;

        let status = response_data["atlanticQuery"]["status"]
            .as_str()
            .ok_or_else(|| Error::InvalidResponse("Missing status field".into()))?;

        Ok(status.to_string())
    }
}

use crate::epoch_update::{EpochProof, EpochUpdate};
use crate::state::{AtlanticJobType, JobStatus, JobType};
use alloy_primitives::FixedBytes;
use std::error::Error;
use tokio_postgres::{Client, NoTls};
use tracing::{error, info};
use uuid::Uuid;

pub struct DatabaseManager {
    client: Client,
}

impl DatabaseManager {
    pub async fn new(db_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let (client, connection) = tokio_postgres::connect(db_url, NoTls).await?;

        // Spawn a task to handle the connection so it is always polled
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Database connection error: {}", e);
            }
        });

        info!("Successfully connected to the database!");

        Ok(Self { client })
    }

    pub async fn insert_verified_epoch(
        &self,
        epoch_id: u64,
        epoch_proof: EpochProof,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO verified_epoch (epoch_id, header_root, state_root, n_signers)
             VALUES ($1, $2, $3, $4)",
                &[
                    &epoch_id.to_string(),
                    &epoch_proof.header_root.to_string(),
                    &epoch_proof.state_root.to_string(),
                    &epoch_proof.n_signers.to_string(),
                    &epoch_proof.execution_hash.to_string(),
                    &epoch_proof.execution_height.to_string(),
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn insert_verified_sync_committee(
        &self,
        sync_committee_id: u64,
        sync_committee_hash: FixedBytes<32>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO verified_sync_committee (sync_committee_id, sync_committee_hash)
             VALUES ($1, $2)",
                &[
                    &sync_committee_id.to_string(),
                    &sync_committee_hash.to_string(),
                ],
            )
            .await?;

        Ok(())
    }

    pub async fn set_atlantic_job_queryid(
        &self,
        job_id: Uuid,
        batch_id: String,
        atlantic_job_type: AtlanticJobType,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match atlantic_job_type {
            AtlanticJobType::ProofGeneration => {
                self.client
                .execute(
                    "UPDATE jobs SET atlantic_proof_generate_batch_id = $1, updated_at = NOW() WHERE job_uuid = $2",
                    &[&batch_id.to_string(), &job_id],
                )
                .await?;
            }
            AtlanticJobType::ProofWrapping => {
                self.client
                .execute(
                    "UPDATE jobs SET atlantic_proof_wrapper_batch_id = $1, updated_at = NOW() WHERE job_uuid = $2",
                    &[&batch_id.to_string(), &job_id],
                )
                .await?;
            } // _ => {
              //     println!("Unk", status);
              // }
        }

        Ok(())
    }
}

use crate::epoch_update::{EpochProof, EpochUpdate};
use crate::state::{AtlanticJobType, JobStatus, JobType, Job};
use alloy_primitives::FixedBytes;
use std::error::Error;
use tokio_postgres::{Client, NoTls};
use tracing::{error, info};
use uuid::Uuid;
use starknet::core::types::Felt;

#[derive(Debug)]
pub struct DatabaseManager {
    client: Client,
}

impl DatabaseManager {
    pub async fn new(db_url: &str) -> Self {
        let client = match tokio_postgres::connect(db_url, tokio_postgres::NoTls).await {
            Ok((client, connection)) => {
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("Connection error: {}", e);
                    }
                });
    
                info!("Connected to the database successfully!");
                client
            }
            Err(err) => {
                error!("Failed to connect to the database: {}", err);
                std::process::exit(1); // Exit with non-zero status code
            }
        };
     
        Self { client }
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

    pub async fn create_job(
        &self,
        job: Job,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO jobs (job_uuid, job_status, slot, type) VALUES ($1, $2, $3, $4)",
                &[
                    &job.job_id,
                    &job.job_status.to_string(),
                    &(job.slot as i64),
                    &"EPOCH_UPDATE",
                ],
            )
            .await?;
    
        Ok(())
    }
    
    pub async fn fetch_job_status(
        &self,
        job_id: Uuid,
    ) -> Result<Option<JobStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let row_opt = self.client
            .query_opt("SELECT status FROM jobs WHERE job_id = $1", &[&job_id])
            .await?;
    
        Ok(row_opt.map(|row| row.get("status")))
    }
    
    pub async fn get_latest_slot_id_in_progress(
        &self,
    ) -> Result<Option<i64>, Box<dyn std::error::Error + Send + Sync>> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        let row_opt = self.client
            .query_opt(
                "SELECT slot FROM jobs
                 WHERE job_status IN ($1, $2)
                 ORDER BY slot DESC
                 LIMIT 1",
                &[&"CREATED", &"PIE_GENERATED"],
            )
            .await?;
    
        // Extract and return the slot ID
        if let Some(row) = row_opt {
            Ok(Some(row.get::<_, i64>("slot")))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_merkle_paths_for_epoch(
        &self,
        epoch_id: i32,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // Query all merkle paths for the given epoch_id
        let rows = self.client
            .query(
                "SELECT merkle_path FROM epoch_merkle_paths
                 WHERE epoch_id = $1
                 ORDER BY path_index ASC",
                &[&epoch_id],
            )
            .await?;
    
        let paths: Vec<String> = rows
            .iter()
            .map(|row| row.get::<_, String>("merkle_path"))
            .collect();
    
        Ok(paths)
    }
    
    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        new_status: JobStatus,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE jobs SET job_status = $1, updated_at = NOW() WHERE job_uuid = $2",
                &[&new_status.to_string(), &job_id],
            )
            .await?;
        Ok(())
    }
    
    pub async fn set_job_txhash(
        &self,
        job_id: Uuid,
        txhash: Felt,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE jobs SET tx_hash = $1, updated_at = NOW() WHERE job_uuid = $2",
                &[&txhash.to_string(), &job_id],
            )
            .await?;
        Ok(())
    }
    
    pub async fn cancell_all_unfinished_jobs(
        &self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE jobs SET status = $1, updated_at = NOW() WHERE status = 'FETCHING'",
                &[&JobStatus::Cancelled.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_merkle_path_for_epoch(
        &self,
        epoch: i32,
        path_index: i32,
        path: String
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO epoch_merkle_paths (epoch_id, path_index, merkle_path) VALUES ($1, $2, $3)",
                &[&epoch, &path_index, &path],
            )
            .await?;
        Ok(())
    }
    
    // async fn fetch_job_by_status(
    //     client: &Client,
    //     status: JobStatus,
    // ) -> Result<Option<Job>, Box<dyn std::error::Error + Send + Sync>> {
    //     let tx = client.transaction().await?;
    
    //     let row_opt = tx
    //         .query_opt(
    //             r#"
    //             SELECT job_id, status
    //             FROM jobs
    //             WHERE status = $1
    //             ORDER BY updated_at ASC
    //             LIMIT 1
    //             FOR UPDATE SKIP LOCKED
    //             "#,
    //             &[&status],
    //         )
    //         .await?;
    
    //     let job = if let Some(row) = row_opt {
    //         Some(Job {
    //             job_id: row.get("job_id"),
    //             job_type: row.get("type"),
    //             job_status: row.get("status"),
    //             slot: row.get("slot"),
    //         })
    //     } else {
    //         None
    //     };
    
    //     tx.commit().await?;
    //     Ok(job)
    // }
    
    // async fn add_verified_epoch(
    //     client: Arc<Client>,
    //     slot: u64,
    // ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //     client
    //         .execute(
    //             "INSERT INTO verified_epochs (slot, job_status, slot, type) VALUES ($1, $2, $3, $4)",
    //             &[&slot, &status.to_string(), &(slot as i64), &"EPOCH_UPDATE"],
    //         )
    //         .await?;
    
    //     Ok(())
    // }
}

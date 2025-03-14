//! Database Manager Implementation
//! 
//! This module provides a PostgreSQL database interface for managing jobs, epochs, and sync committees.
//! It handles all database operations including job tracking, status updates, and verification data storage.
//! The manager provides a robust interface for tracking the state of various blockchain operations.

use std::{collections::HashMap, str::FromStr};

use chrono::NaiveDateTime;
use num_traits::ToPrimitive;
use thiserror::Error;
use tokio_postgres::{Client, Row};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use alloy_primitives::{
    hex::{FromHex, ToHexExt},
    FixedBytes,
};
use serde::Serialize;

use starknet::core::types::Felt;

use crate::{
    types::job::{AtlanticJobType, Job, JobStatus, JobType},
    types::proofs::epoch_update::{EpochDecommitmentData, EpochProof, ExpectedEpochUpdateOutputs},
    utils::helpers,
    utils::merkle::MerklePath,
};

/// Schema for job data stored in the database
#[derive(Debug, Serialize)]
pub struct JobSchema {
    /// Unique identifier for the job
    pub job_uuid: uuid::Uuid,
    /// Current status of the job
    pub job_status: JobStatus,
    /// Slot number associated with the job
    pub slot: i64,
    /// Starting epoch for batch operations
    pub batch_range_begin_epoch: i64,
    /// Ending epoch for batch operations
    pub batch_range_end_epoch: i64,
    /// Type of job (epoch update or sync committee update)
    pub job_type: JobType,
    /// Transaction hash if the job has been submitted
    pub tx_hash: Option<String>,
    /// Atlantic proof generation batch ID
    pub atlantic_proof_generate_batch_id: Option<String>,
    /// Atlantic proof wrapper batch ID
    pub atlantic_proof_wrapper_batch_id: Option<String>,
    /// Status at which the job failed, if applicable
    pub failed_at_step: Option<JobStatus>,
    /// Number of retry attempts
    pub retries_count: Option<i64>,
    /// Timestamp of the last failure
    pub last_failure_time: Option<NaiveDateTime>,
}

/// Extended job information including timestamps
#[derive(Debug)]
pub struct JobWithTimestamps {
    /// Job schema information
    pub job: JobSchema,
    /// Creation timestamp
    pub created_at: String,
    /// Last update timestamp
    pub updated_at: String,
    /// Transaction hash if available
    pub tx_hash: Option<String>,
}

/// Count of jobs in a particular status
pub struct JobStatusCount {
    /// Job status
    pub status: JobStatus,
    /// Number of jobs in this status
    pub count: i64,
}

/// Main database manager for handling all database operations
#[derive(Debug)]
pub struct DatabaseManager {
    /// PostgreSQL client connection
    client: Client,
}

impl DatabaseManager {
    /// Creates a new database manager instance
    ///
    /// # Arguments
    /// * `db_url` - PostgreSQL connection URL
    ///
    /// # Returns
    /// * `Self` - New database manager instance
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

    /// Inserts a verified epoch into the database
    ///
    /// # Arguments
    /// * `epoch_id` - ID of the epoch
    /// * `epoch_proof` - Proof data for the epoch
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn insert_verified_epoch(
        &self,
        epoch_id: u64,
        epoch_proof: EpochProof,
    ) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "INSERT INTO verified_epoch (epoch_id, header_root, state_root, n_signers)
             VALUES ($1, $2, $3, $4, $4, $6)",
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

    /// Inserts verified epoch decommitment data
    ///
    /// # Arguments
    /// * `epoch_id` - ID of the epoch
    /// * `beacon_header_root` - Root hash of the beacon header
    /// * `beacon_state_root` - Root hash of the beacon state
    /// * `slot` - Slot number
    /// * `committee_hash` - Hash of the committee
    /// * `n_signers` - Number of signers
    /// * `execution_header_hash` - Hash of the execution header
    /// * `execution_header_height` - Height of the execution header
    /// * `epoch_index` - Index of the epoch
    /// * `batch_root` - Root hash of the batch
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn insert_verified_epoch_decommitment_data(
        &self,
        epoch_id: u64,
        beacon_header_root: FixedBytes<32>,
        beacon_state_root: FixedBytes<32>,
        slot: u64,
        committee_hash: FixedBytes<32>,
        n_signers: u64,
        execution_header_hash: FixedBytes<32>,
        execution_header_height: u64,
        epoch_index: usize,
        batch_root: Felt,
    ) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "INSERT INTO verified_epoch (epoch_id, beacon_header_root, beacon_state_root, slot, committee_hash, n_signers, execution_header_hash, execution_header_height, epoch_index, batch_root)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                &[
                    &epoch_id.to_i64(),
                    &beacon_header_root.encode_hex_with_prefix(),
                    &beacon_state_root.encode_hex_with_prefix(),
                    &slot.to_i64(),
                    &committee_hash.encode_hex_with_prefix(),
                    &n_signers.to_i64(),
                    &execution_header_hash.encode_hex_with_prefix(),
                    &execution_header_height.to_i64(),
                    &epoch_index.to_i64(),
                    &batch_root.to_hex_string(),
                ],
            )
            .await?;

        Ok(())
    }

    /// Inserts a verified sync committee
    ///
    /// # Arguments
    /// * `sync_committee_id` - ID of the sync committee
    /// * `sync_committee_hash` - Hash of the sync committee
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn insert_verified_sync_committee(
        &self,
        sync_committee_id: u64,
        sync_committee_hash: String,
    ) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "INSERT INTO verified_sync_committee (sync_committee_id, sync_committee_hash)
             VALUES ($1, $2)",
                &[&sync_committee_id.to_i64(), &sync_committee_hash],
            )
            .await?;

        Ok(())
    }

    /// Sets the Atlantic job query ID
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    /// * `atlantic_batch_job_id` - Atlantic batch job ID
    /// * `atlantic_job_type` - Type of Atlantic job
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_atlantic_job_queryid(
        &self,
        job_id: Uuid,
        atlantic_batch_job_id: String,
        atlantic_job_type: AtlanticJobType,
    ) -> Result<(), DatabaseError> {
        match atlantic_job_type {
            AtlanticJobType::ProofGeneration => {
                self.client
                .execute(
                    "UPDATE jobs SET atlantic_proof_generate_batch_id = $1, updated_at = NOW() WHERE job_uuid = $2",
                    &[&atlantic_batch_job_id.to_string(), &job_id],
                )
                .await?;
            }
            AtlanticJobType::ProofWrapping => {
                self.client
                .execute(
                    "UPDATE jobs SET atlantic_proof_wrapper_batch_id = $1, updated_at = NOW() WHERE job_uuid = $2",
                    &[&atlantic_batch_job_id.to_string(), &job_id],
                )
                .await?;
            } // _ => {
              //     println!("Unk", status);
              // }
        }

        Ok(())
    }

    /// Creates a new job in the database
    ///
    /// # Arguments
    /// * `job` - Job data to create
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn create_job(&self, job: Job) -> Result<(), DatabaseError> {
        match job.job_type {
            JobType::EpochBatchUpdate => {
                self.client
                    .execute(
                        "INSERT INTO jobs (job_uuid, job_status, slot, type, batch_range_begin_epoch, batch_range_end_epoch) VALUES ($1, $2, $3, $4, $5, $6)",
                        &[
                            &job.job_id,
                            &job.job_status.to_string(),
                            &(job.slot.unwrap() as i64),
                            &"EPOCH_BATCH_UPDATE",
                            &(job.batch_range_begin_epoch.unwrap() as i64),
                            &(job.batch_range_end_epoch.unwrap() as i64),
                        ],
                    )
                    .await?;
            }
            //JobType::EpochUpdate => {}
            JobType::SyncCommitteeUpdate => {
                self.client
                    .execute(
                        "INSERT INTO jobs (job_uuid, job_status, slot, type) VALUES ($1, $2, $3, $4)",
                        &[
                            &job.job_id,
                            &job.job_status.to_string(),
                            &(job.slot.unwrap() as i64),
                            &"SYNC_COMMITTEE_UPDATE",
                        ],
                    )
                    .await?;
            }
        }

        Ok(())
    }

    /// Fetches the status of a job
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    ///
    /// # Returns
    /// * `Result<Option<JobStatus>, DatabaseError>` - Job status or error
    pub async fn fetch_job_status(&self, job_id: Uuid) -> Result<Option<JobStatus>, DatabaseError> {
        let row_opt = self
            .client
            .query_opt("SELECT status FROM jobs WHERE job_uuid = $1", &[&job_id])
            .await?;

        Ok(row_opt.map(|row| row.get("status")))
    }

    /// Gets a job by its ID
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    ///
    /// # Returns
    /// * `Result<Option<JobSchema>, DatabaseError>` - Job data or error
    pub async fn get_job_by_id(&self, job_id: Uuid) -> Result<Option<JobSchema>, DatabaseError> {
        let row_opt = self
            .client
            .query_opt("SELECT * FROM jobs WHERE job_uuid = $1", &[&job_id])
            .await?;
        let job = row_opt.map(Self::map_row_to_job).transpose()?;
        Ok(job)
    }

    /// Gets the latest epoch that is in progress
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Latest epoch ID or error
    pub async fn get_latest_epoch_in_progress(&self) -> Result<Option<u64>, DatabaseError> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        // //, 'CANCELLED', 'ERROR'
        let row_opt = self
            .client
            .query_opt(
                "SELECT batch_range_end_epoch FROM jobs
                 WHERE job_status NOT IN ('DONE')
                        AND batch_range_end_epoch != 0
                        AND type = 'EPOCH_BATCH_UPDATE'
                 ORDER BY batch_range_end_epoch DESC
                 LIMIT 1",
                &[],
            )
            .await?;

        // Extract and return the slot ID
        if let Some(row) = row_opt {
            let value = row.get::<_, i64>("batch_range_end_epoch");
            Ok(Some(value.to_u64().ok_or(
                DatabaseError::IntegerConversion(value.to_string()),
            )?))
        } else {
            Ok(Some(0))
        }
    }

    /// Gets the latest completed epoch
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Latest completed epoch ID or error
    pub async fn get_latest_done_epoch(&self) -> Result<Option<u64>, DatabaseError> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        // //, 'CANCELLED', 'ERROR'
        let row_opt = self
            .client
            .query_opt(
                "SELECT batch_range_end_epoch FROM jobs
                 WHERE job_status = 'DONE'
                        AND batch_range_end_epoch != 0
                        AND type = 'EPOCH_BATCH_UPDATE'
                 ORDER BY batch_range_end_epoch DESC
                 LIMIT 1",
                &[],
            )
            .await?;

        // Extract and return the slot ID
        if let Some(row) = row_opt {
            let value = row.get::<_, i64>("batch_range_end_epoch");
            Ok(Some(value.to_u64().ok_or(
                DatabaseError::IntegerConversion(value.to_string()),
            )?))
        } else {
            Ok(Some(0))
        }
    }

    /// Gets the latest sync committee that is in progress
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Latest sync committee ID or error
    pub async fn get_latest_sync_committee_in_progress(
        &self,
    ) -> Result<Option<u64>, DatabaseError> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        let row_opt = self
            .client
            .query_opt(
                "SELECT slot FROM jobs
                 WHERE job_status NOT IN ('DONE')
                        AND type = 'SYNC_COMMITTEE_UPDATE'
                 ORDER BY slot DESC
                 LIMIT 1",
                &[],
            )
            .await?;

        // Extract and return the slot ID
        if let Some(row) = row_opt {
            let value = row.get::<_, i64>("slot");
            Ok(Some(helpers::slot_to_sync_committee_id(
                value
                    .to_u64()
                    .ok_or(DatabaseError::IntegerConversion(value.to_string()))?,
            )))
        } else {
            Ok(Some(0))
        }
    }

    /// Gets the latest completed sync committee
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Latest completed sync committee ID or error
    pub async fn get_latest_done_sync_committee(&self) -> Result<Option<u64>, DatabaseError> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        let row_opt = self
            .client
            .query_opt(
                "SELECT slot FROM jobs
                 WHERE job_status = 'DONE'
                        AND type = 'SYNC_COMMITTEE_UPDATE'
                 ORDER BY slot DESC
                 LIMIT 1",
                &[],
            )
            .await?;

        // Extract and return the slot ID
        if let Some(row) = row_opt {
            let value = row.get::<_, i64>("slot");
            Ok(Some(helpers::slot_to_sync_committee_id(
                value
                    .to_u64()
                    .ok_or(DatabaseError::IntegerConversion(value.to_string()))?,
            )))
        } else {
            Ok(Some(0))
        }
    }

    /// Counts jobs that are currently in progress
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Count of in-progress jobs or error
    pub async fn count_jobs_in_progress(&self) -> Result<Option<u64>, DatabaseError> {
        // Query the latest slot with job_status in ('in_progress', 'initialized')
        let row_opt = self
            .client
            .query_opt(
                "SELECT COUNT(job_uuid) as count FROM jobs
                 WHERE job_status NOT IN ('DONE', 'CANCELLED', 'ERROR')
                        AND type = 'EPOCH_BATCH_UPDATE'
                ",
                &[],
            )
            .await?;

        // Extract and return the slot ID
        if let Some(row) = row_opt {
            let value = row.get::<_, i64>("count");
            Ok(Some(value.to_u64().ok_or(
                DatabaseError::IntegerConversion(value.to_string()),
            )?))
        } else {
            Ok(Some(0))
        }
    }

    /// Gets merkle paths for a specific epoch
    ///
    /// # Arguments
    /// * `epoch_id` - ID of the epoch
    ///
    /// # Returns
    /// * `Result<Vec<MerklePath>, DatabaseError>` - Merkle paths or error
    pub async fn get_merkle_paths_for_epoch(
        &self,
        epoch_id: i32,
    ) -> Result<Vec<MerklePath>, DatabaseError> {
        // Query all merkle paths for the given epoch_id
        let rows = self
            .client
            .query(
                "SELECT path_index, merkle_path
                 FROM epoch_merkle_paths
                 WHERE epoch_id = $1
                 ORDER BY path_index ASC",
                &[&epoch_id.to_i64()],
            )
            .await?;

        let paths: Vec<MerklePath> = rows
            .iter()
            .map(|row| MerklePath {
                leaf_index: row.get::<_, i64>("path_index").to_u64().unwrap(),
                value: Felt::from_hex(row.get("merkle_path")).unwrap(),
            })
            .collect();

        Ok(paths)
    }

    /// Gets epoch decommitment data
    ///
    /// # Arguments
    /// * `epoch_id` - ID of the epoch
    ///
    /// # Returns
    /// * `Result<EpochDecommitmentData, DatabaseError>` - Decommitment data or error
    pub async fn get_epoch_decommitment_data(
        &self,
        epoch_id: i32,
    ) -> Result<EpochDecommitmentData, DatabaseError> {
        let row = self
            .client
            .query_one(
                r#"
                SELECT
                    beacon_header_root,
                    beacon_state_root,
                    slot,
                    committee_hash,
                    n_signers,                    execution_header_hash,
                    execution_header_height,
                    batch_root,
                    epoch_index
                FROM verified_epoch
                WHERE epoch_id = $1
                "#,
                &[&epoch_id.to_i64()],
            )
            .await?;

        Ok(EpochDecommitmentData {
            epoch_update_outputs: ExpectedEpochUpdateOutputs {
                beacon_header_root: FixedBytes::from_hex(
                    row.get::<_, String>("beacon_header_root"),
                )
                .unwrap(),
                beacon_state_root: FixedBytes::from_hex(row.get::<_, String>("beacon_state_root"))
                    .unwrap(),
                slot: row.get::<_, i64>("slot") as u64,
                committee_hash: FixedBytes::from_hex(row.get::<_, String>("committee_hash"))
                    .unwrap(),
                n_signers: row.get::<_, i64>("n_signers") as u64,
                execution_header_hash: FixedBytes::from_hex(
                    row.get::<_, String>("execution_header_hash"),
                )
                .unwrap(),
                execution_header_height: row.get::<_, i64>("execution_header_height") as u64,
            },
            batch_root: Felt::from_hex(row.get::<_, &str>("batch_root")).unwrap(),
            epoch_index: row.get::<_, i64>("epoch_index") as u64,
        })
    }

    /// Gets all jobs with a specific status
    ///
    /// # Arguments
    /// * `desired_status` - Status to filter by
    ///
    /// # Returns
    /// * `Result<Vec<JobSchema>, DatabaseError>` - Matching jobs or error
    pub async fn get_jobs_with_status(
        &self,
        desired_status: JobStatus,
    ) -> Result<Vec<JobSchema>, DatabaseError> {
        // Query all jobs with the given job_status
        let rows = self
            .client
            .query(
                "SELECT * FROM jobs
                 WHERE job_status = $1",
                &[&desired_status.to_string()],
            )
            .await?;

        let jobs = rows
            .iter()
            .cloned()
            .map(Self::map_row_to_job)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(jobs)
    }

    /// Counts jobs with a specific status
    ///
    /// # Arguments
    /// * `desired_status` - Status to count
    ///
    /// # Returns
    /// * `Result<u64, DatabaseError>` - Count of jobs or error
    pub async fn count_jobs_with_status(
        &self,
        desired_status: JobStatus,
    ) -> Result<u64, DatabaseError> {
        // Query all jobs with the given job_status
        let row = self
            .client
            .query_one(
                "SELECT  COUNT(*) FROM jobs
                 WHERE job_status = $1",
                &[&desired_status.to_string()],
            )
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    /// Updates the status of a job
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    /// * `new_status` - New status to set
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        new_status: JobStatus,
    ) -> Result<(), DatabaseError> {
        info!(
            "Job {} status changed to {}",
            job_id,
            new_status.to_string()
        );
        self.client
            .execute(
                "UPDATE jobs SET job_status = $1, updated_at = NOW() WHERE job_uuid = $2",
                &[&new_status.to_string(), &job_id],
            )
            .await?;
        Ok(())
    }

    /// Sets failure information for a job
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    /// * `failed_at_step` - Step at which the job failed
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_failure_info(
        &self,
        job_id: Uuid,
        failed_at_step: JobStatus,
    ) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "UPDATE jobs SET failed_at_step = $1, updated_at = NOW(), last_failure_time = NOW() WHERE job_uuid = $2",
                &[&failed_at_step.to_string(), &job_id],
            )
            .await?;
        Ok(())
    }

    /// Counts epoch jobs waiting for sync committee update
    ///
    /// # Arguments
    /// * `latest_verified_sync_committee` - Latest verified sync committee ID
    ///
    /// # Returns
    /// * `Result<u64, DatabaseError>` - Count of waiting jobs or error
    pub async fn count_epoch_jobs_waiting_for_sync_committe_update(
        &self,
        latest_verified_sync_committee: u64,
    ) -> Result<u64, DatabaseError> {
        let epoch_to_start_check_from =
            helpers::get_last_epoch_for_sync_committee(latest_verified_sync_committee) + 1; // So we getting first epoch number from latest unverified committee
        let row = self
            .client
            .query_one(
                "SELECT COUNT(*) as count FROM jobs WHERE batch_range_begin_epoch >= $1
                 AND job_status = 'OFFCHAIN_COMPUTATION_FINISHED'",
                &[&epoch_to_start_check_from.to_i64()],
            )
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    /// Sets batch epochs as ready to broadcast
    ///
    /// # Arguments
    /// * `first_epoch` - First epoch in range
    /// * `last_epoch` - Last epoch in range
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_ready_to_broadcast_for_batch_epochs(
        &self,
        first_epoch: u64,
        last_epoch: u64,
    ) -> Result<(), DatabaseError> {
        let rows_affected = self.client
            .execute(
                "UPDATE jobs
                SET job_status = 'READY_TO_BROADCAST_ONCHAIN', updated_at = NOW()
                WHERE batch_range_begin_epoch >= $1 AND batch_range_end_epoch <= $2 AND type = 'EPOCH_BATCH_UPDATE'
                      AND job_status = 'OFFCHAIN_COMPUTATION_FINISHED'",
                &[&first_epoch.to_i64(), &last_epoch.to_i64()],
            )
            .await?;

        if rows_affected > 0 {
            info!(
                "{} EPOCH_BATCH_UPDATE jobs changed state to READY_TO_BROADCAST_ONCHAIN",
                rows_affected
            );
        }
        Ok(())
    }

    /// Sets batch epochs as ready to broadcast up to a specific epoch
    ///
    /// # Arguments
    /// * `to_epoch` - Upper bound epoch
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_ready_to_broadcast_for_batch_epochs_to(
        &self,
        to_epoch: u64,
    ) -> Result<(), DatabaseError> {
        let rows_affected = self
            .client
            .execute(
                "UPDATE jobs
                SET job_status = 'READY_TO_BROADCAST_ONCHAIN', updated_at = NOW()
                WHERE batch_range_end_epoch <= $1 AND type = 'EPOCH_BATCH_UPDATE'
                      AND job_status = 'OFFCHAIN_COMPUTATION_FINISHED'",
                &[&to_epoch.to_i64()],
            )
            .await?;

        if rows_affected > 0 {
            info!(
                "{} EPOCH_BATCH_UPDATE jobs changed state to READY_TO_BROADCAST_ONCHAIN",
                rows_affected
            );
        }
        Ok(())
    }

    /// Sets sync committee as ready to broadcast
    ///
    /// # Arguments
    /// * `sync_committee_id` - ID of the sync committee
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_ready_to_broadcast_for_sync_committee(
        &self,
        sync_committee_id: u64,
    ) -> Result<(), DatabaseError> {
        let sync_commite_first_slot = helpers::get_first_slot_for_sync_committee(sync_committee_id);
        let sync_commite_last_slot = helpers::get_last_slot_for_sync_committee(sync_committee_id);

        debug!(
            "Setting syn committee between slots {} and {} to READY_TO_BROADCAST_ONCHAIN",
            sync_commite_first_slot, sync_commite_last_slot
        );

        let rows_affected = self
            .client
            .execute(
                "UPDATE jobs
                SET job_status = 'READY_TO_BROADCAST_ONCHAIN', updated_at = NOW()
                WHERE type = 'SYNC_COMMITTEE_UPDATE'
                AND job_status = 'OFFCHAIN_COMPUTATION_FINISHED'
                AND slot BETWEEN $1 AND $2
                ",
                &[
                    &sync_commite_first_slot.to_i64(),
                    &sync_commite_last_slot.to_i64(),
                ],
            )
            .await?;

        if rows_affected == 1 {
            info!(
                "{} SYNC_COMMITTEE_UPDATE jobs changed state to READY_TO_BROADCAST_ONCHAIN",
                rows_affected
            );
        } else if rows_affected > 1 {
            warn!(
                "{} SYNC_COMMITTEE_UPDATE jobs changed state to READY_TO_BROADCAST_ONCHAIN in one query, something may be wrong!",
                rows_affected
            );
        }
        Ok(())
    }

    /// Sets the transaction hash for a job
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    /// * `txhash` - Transaction hash
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn set_job_txhash(&self, job_id: Uuid, txhash: Felt) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "UPDATE jobs SET tx_hash = $1, updated_at = NOW() WHERE job_uuid = $2",
                &[&txhash.to_hex_string(), &job_id],
            )
            .await?;
        Ok(())
    }

    /// Increments the retry counter for a job
    ///
    /// # Arguments
    /// * `job_id` - UUID of the job
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn increment_job_retry_counter(&self, job_id: Uuid) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "UPDATE jobs SET retries_count = COALESCE(retries_count, 0) + 1, updated_at = NOW() WHERE job_uuid = $1",
                &[ &job_id],
            )
            .await?;
        Ok(())
    }

    /// Inserts a merkle path for an epoch
    ///
    /// # Arguments
    /// * `epoch` - Epoch ID
    /// * `path_index` - Index of the path
    /// * `path` - Merkle path string
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn insert_merkle_path_for_epoch(
        &self,
        epoch: u64,
        path_index: u64,
        path: String,
    ) -> Result<(), DatabaseError> {
        let rows_affected =self.client
            .execute(
                "INSERT INTO epoch_merkle_paths (epoch_id, path_index, merkle_path) VALUES ($1, $2, $3)
                 ON CONFLICT (epoch_id, path_index) DO NOTHING",
                &[&epoch.to_i64(), &path_index.to_i64(), &path],
            )
            .await?;

        if rows_affected == 0 {
            warn!("Combination of epoch_id and path_index already exists, skipping insertion of epoch merkle patch for epoch {} and index {}", epoch, path_index);
        }
        Ok(())
    }

    /// Gets jobs with specific statuses
    ///
    /// # Arguments
    /// * `desired_statuses` - List of statuses to filter by
    ///
    /// # Returns
    /// * `Result<Vec<JobSchema>, DatabaseError>` - Matching jobs or error
    pub async fn get_jobs_with_statuses(
        &self,
        desired_statuses: Vec<JobStatus>,
    ) -> Result<Vec<JobSchema>, DatabaseError> {
        if desired_statuses.is_empty() {
            return Ok(vec![]);
        }

        let status_strings: Vec<String> = desired_statuses.iter().map(|s| s.to_string()).collect();

        let placeholders: Vec<String> = (1..=status_strings.len())
            .map(|i| format!("${}", i))
            .collect();
        let query = format!(
            "SELECT * FROM jobs WHERE job_status IN ({})",
            placeholders.join(", ")
        );

        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = status_strings
            .iter()
            .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        let rows = self.client.query(&query, &params).await?;

        let jobs = rows
            .iter()
            .cloned()
            .map(Self::map_row_to_job)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(jobs)
    }

    /// Updates daemon state information
    ///
    /// # Arguments
    /// * `latest_known_beacon_slot` - Latest known beacon slot
    /// * `latest_known_beacon_block` - Latest known beacon block hash
    ///
    /// # Returns
    /// * `Result<(), DatabaseError>` - Success or error
    pub async fn update_daemon_state_info(
        &self,
        latest_known_beacon_slot: u64,
        latest_known_beacon_block: FixedBytes<32>,
    ) -> Result<(), DatabaseError> {
        self.client
            .execute(
                "UPDATE daemon_state SET latest_known_beacon_slot = $1, latest_known_beacon_block = NOW()",
                &[&latest_known_beacon_slot.to_string(), &latest_known_beacon_block.to_string()],
            )
            .await?;
        Ok(())
    }

    /// Gets the total count of jobs
    ///
    /// # Returns
    /// * `Result<u64, Box<dyn std::error::Error + Send + Sync>>` - Total job count or error
    pub async fn count_total_jobs(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let row = self
            .client
            .query_one("SELECT COUNT(*) as count FROM jobs WHERE job_status = 'DONE' OR job_status = 'ERROR'", &[])
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    /// Gets the count of successful jobs
    ///
    /// # Returns
    /// * `Result<u64, DatabaseError>` - Successful job count or error
    pub async fn count_successful_jobs(&self) -> Result<u64, DatabaseError> {
        let row = self
            .client
            .query_one(
                "SELECT COUNT(*) as count FROM jobs WHERE job_status = 'DONE'",
                &[],
            )
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    /// Gets the average job duration
    ///
    /// # Returns
    /// * `Result<i64, DatabaseError>` - Average duration in seconds or error
    pub async fn get_average_job_duration(&self) -> Result<i64, DatabaseError> {
        let row = self
            .client
            .query_one(
                "SELECT EXTRACT(EPOCH FROM AVG(updated_at - created_at))::INTEGER AS avg_duration
                 FROM (                    
                    SELECT updated_at, created_at
                    FROM jobs
                    WHERE job_status = 'DONE'
                    ORDER BY updated_at DESC
                    LIMIT 20
                 ) subquery",
                &[],
            )
            .await?;

        Ok(i64::from(
            row.get::<_, Option<i32>>("avg_duration").unwrap_or(0),
        ))
    }

    /// Gets recent batch jobs
    ///
    /// # Arguments
    /// * `limit` - Maximum number of jobs to return
    ///
    /// # Returns
    /// * `Result<Vec<JobWithTimestamps>, DatabaseError>` - Recent jobs or error
    pub async fn get_recent_batch_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, DatabaseError> {
        let rows = self
            .client
            .query(
                "SELECT *,
                 to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_time,
                 to_char(updated_at, 'YYYY-MM-DD HH24:MI:SS') as updated_time
                 FROM jobs
                 WHERE type = 'EPOCH_BATCH_UPDATE'
                 ORDER BY batch_range_begin_epoch DESC
                 LIMIT $1",
                &[&limit],
            )
            .await?;

        let jobs = rows
            .into_iter()
            .map(|row| {
                let job = Self::map_row_to_job(row.clone())?;
                Ok(JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                })
            })
            .collect::<Result<Vec<JobWithTimestamps>, DatabaseError>>()?;

        Ok(jobs)
    }

    /// Gets recent sync committee jobs
    ///
    /// # Arguments
    /// * `limit` - Maximum number of jobs to return
    ///
    /// # Returns
    /// * `Result<Vec<JobWithTimestamps>, DatabaseError>` - Recent jobs or error
    pub async fn get_recent_sync_committee_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, DatabaseError> {
        let rows = self
            .client
            .query(
                "SELECT *,
                 to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_time,
                 to_char(updated_at, 'YYYY-MM-DD HH24:MI:SS') as updated_time
                 FROM jobs
                 WHERE type = 'SYNC_COMMITTEE_UPDATE'
                 ORDER BY slot DESC
                 LIMIT $1",
                &[&limit],
            )
            .await?;

        let jobs = rows
            .into_iter()
            .map(|row| {
                let job = Self::map_row_to_job(row.clone())?;
                Ok(JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                })
            })
            .collect::<Result<Vec<JobWithTimestamps>, DatabaseError>>()?;

        Ok(jobs)
    }

    /// Checks if the database connection is alive
    ///
    /// # Returns
    /// * `bool` - True if connected, false otherwise
    pub async fn is_connected(&self) -> bool {
        self.client.query_one("SELECT 1", &[]).await.is_ok()
    }

    /// Gets recent Atlantic queries in progress
    ///
    /// # Arguments
    /// * `limit` - Maximum number of queries to return
    ///
    /// # Returns
    /// * `Result<Vec<JobWithTimestamps>, DatabaseError>` - Recent queries or error
    pub async fn get_recent_atlantic_queries_in_progress(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, DatabaseError> {
        let rows = self
            .client
            .query(
                "SELECT atlantic_proof_generate_batch_id, atlantic_proof_wrapper_batch_id
                 FROM jobs
                 WHERE job_status != 'DONE'
                 ORDER BY slot DESC
                 LIMIT $1",
                &[&limit],
            )
            .await?;

        let jobs = rows
            .into_iter()
            .map(|row| {
                let job = Self::map_row_to_job(row.clone())?;
                Ok(JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                })
            })
            .collect::<Result<Vec<JobWithTimestamps>, DatabaseError>>()?;

        Ok(jobs)
    }

    /// Gets verified epoch by execution height
    ///
    /// # Arguments
    /// * `execution_header_height` - Execution header height
    ///
    /// # Returns
    /// * `Result<Option<u64>, DatabaseError>` - Epoch ID or error
    pub async fn get_verified_epoch_by_execution_height(
        &self,
        execution_header_height: i32,
    ) -> Result<Option<u64>, DatabaseError> {
        let row_opt = self
            .client
            .query_opt(
                "SELECT epoch_id FROM verified_epoch WHERE execution_header_height = $1
                 LIMIT 1",
                &[&execution_header_height],
            )
            .await?;

        if let Some(row) = row_opt {
            Ok(Some(row.get::<_, i64>("epoch_id").to_u64().unwrap()))
        } else {
            Ok(Some(0))
        }
    }

    /// Gets count of jobs by status
    ///
    /// # Returns
    /// * `Result<Vec<JobStatusCount>, DatabaseError>` - Job counts by status or error
    pub async fn get_jobs_count_by_status(&self) -> Result<Vec<JobStatusCount>, DatabaseError> {
        let rows = self
            .client
            .query(
                "SELECT job_status, COUNT(*) AS job_count FROM jobs GROUP BY job_status",
                &[],
            )
            .await?;

        let mut db_counts: HashMap<JobStatus, i64> = HashMap::new();
        for row in rows {
            let status_str: String = row.get("job_status");
            let status_count: i64 = row.get("job_count");

            let job_status = JobStatus::from_str(&status_str)?;
            db_counts.insert(job_status, status_count);
        }

        let all_possible_statuses = vec![
            JobStatus::Created,
            JobStatus::StartedFetchingInputs,
            JobStatus::ProgramInputsPrepared,
            JobStatus::StartedTraceGeneration,
            JobStatus::PieGenerated,
            JobStatus::AtlanticProofRequested,
            JobStatus::AtlanticProofRetrieved,
            JobStatus::WrapProofRequested,
            JobStatus::WrappedProofDone,
            JobStatus::OffchainComputationFinished,
            JobStatus::ReadyToBroadcastOnchain,
            JobStatus::ProofVerifyCalledOnchain,
            JobStatus::Done,
            JobStatus::Error,
            JobStatus::Cancelled,
        ];

        let mut result = Vec::with_capacity(all_possible_statuses.len());
        for status in all_possible_statuses {
            let count = db_counts.get(&status).copied().unwrap_or(0);
            result.push(JobStatusCount { status, count });
        }

        Ok(result)
    }

    // Helper functions
    fn map_row_to_job(row: Row) -> Result<JobSchema, DatabaseError> {
        let job_status_str: String = row.get("job_status");
        let job_status = JobStatus::from_str(&job_status_str)?;

        let job_type_str: String = row.get("type");
        let job_type = JobType::from_str(&job_type_str)?;

        let failed_at_step: Option<JobStatus> = row
            .get::<_, Option<String>>("failed_at_step")
            .map(|step| JobStatus::from_str(&step))
            .transpose()?;

        let last_failure_time: Option<NaiveDateTime> = row.get("last_failure_time");

        Ok(JobSchema {
            job_uuid: row.get("job_uuid"),
            job_status,
            slot: row.get("slot"),
            batch_range_begin_epoch: row
                .get::<&str, Option<i64>>("batch_range_begin_epoch")
                .unwrap_or(0),
            batch_range_end_epoch: row
                .get::<&str, Option<i64>>("batch_range_end_epoch")
                .unwrap_or(0),
            job_type,
            tx_hash: row.get("tx_hash"),
            atlantic_proof_generate_batch_id: row.get("atlantic_proof_generate_batch_id"),
            atlantic_proof_wrapper_batch_id: row.get("atlantic_proof_wrapper_batch_id"),
            failed_at_step,
            retries_count: row.get("retries_count"),
            last_failure_time,
        })
    }
}

/// Possible errors that can occur during database operations
#[derive(Debug, Error)]
pub enum DatabaseError {
    /// PostgreSQL connection or query error
    #[error("Database connection error: {0}")]
    Postgres(#[from] tokio_postgres::Error),
    /// Error mapping database row to job
    #[error("Job mapping error: {0}")]
    JobMapping(#[from] Box<dyn std::error::Error + Send + Sync>),
    /// Error converting between integer types
    #[error("Integer conversion error: {0}")]
    IntegerConversion(String),
    /// General parsing error
    #[error("Parse error: {0}")]
    Parse(String),
    /// Error parsing string into enum
    #[error("String parsing error: {0}")]
    StrumParse(#[from] strum::ParseError),
}

impl From<String> for DatabaseError {
    fn from(err: String) -> Self {
        DatabaseError::Parse(err)
    }
}

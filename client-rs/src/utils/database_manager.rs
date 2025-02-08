use crate::helpers;
use crate::state::{AtlanticJobType, Error, Job, JobStatus, JobType};
use crate::utils::starknet_client::EpochProof;
use alloy_primitives::FixedBytes;
use starknet::core::types::Felt;
use std::str::FromStr;
//use std::error::Error;
use chrono::NaiveDateTime;
use num_traits::ToPrimitive;
use std::collections::HashMap;
use tokio_postgres::{Client, Row};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Debug)]
pub struct JobSchema {
    pub job_uuid: uuid::Uuid,
    pub job_status: JobStatus,
    pub slot: i64,
    pub batch_range_begin_epoch: i64,
    pub batch_range_end_epoch: i64,
    pub job_type: JobType,
    pub atlantic_proof_generate_batch_id: Option<String>,
    pub atlantic_proof_wrapper_batch_id: Option<String>,
    pub failed_at_step: Option<JobStatus>,
    pub retries_count: Option<i64>,
    pub last_failure_time: Option<NaiveDateTime>, //pub updated_at: i64,
}

#[derive(Debug)]
pub struct JobWithTimestamps {
    pub job: JobSchema,
    pub created_at: String,
    pub updated_at: String,
    pub tx_hash: Option<String>,
}

pub struct JobStatusCount {
    pub status: JobStatus,
    pub count: i64,
}

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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn insert_verified_sync_committee(
        &self,
        sync_committee_id: u64,
        sync_committee_hash: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "INSERT INTO verified_sync_committee (sync_committee_id, sync_committee_hash)
             VALUES ($1, $2)",
                &[&sync_committee_id.to_string(), &sync_committee_hash],
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
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;
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
                    .await
                    .map_err(|e| Error::DatabaseError(e.to_string()))?;
            }
        }

        Ok(())
    }

    pub async fn fetch_job_status(
        &self,
        job_id: Uuid,
    ) -> Result<Option<JobStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let row_opt = self
            .client
            .query_opt("SELECT status FROM jobs WHERE job_uuid = $1", &[&job_id])
            .await?;

        Ok(row_opt.map(|row| row.get("status")))
    }

    pub async fn get_job_by_id(
        &self,
        job_id: Uuid,
    ) -> Result<Option<JobSchema>, Box<dyn std::error::Error + Send + Sync>> {
        let row_opt = self
            .client
            .query_opt("SELECT * FROM jobs WHERE job_uuid = $1", &[&job_id])
            .await?;

        row_opt.map(Self::map_row_to_job).transpose()
    }
    // pub async fn get_latest_slot_id_in_progress(
    //     &self,
    // ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
    //     // Query the latest slot with job_status in ('in_progress', 'initialized')
    //     let row_opt = self
    //         .client
    //         .query_opt(
    //             "SELECT slot FROM jobs
    //              WHERE job_status NOT IN ('DONE', 'CANCELLED', 'ERROR')
    //              ORDER BY slot DESC
    //              LIMIT 1",
    //             &[],
    //         )
    //         .await?;

    //     // Extract and return the slot ID
    //     if let Some(row) = row_opt {
    //         Ok(Some(row.get::<_, i64>("slot").to_u64().unwrap()))
    //     } else {
    //         Ok(Some(0))
    //     }
    // }

    pub async fn get_latest_epoch_in_progress(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
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
            Ok(Some(
                row.get::<_, i64>("batch_range_end_epoch").to_u64().unwrap(),
            ))
        } else {
            Ok(Some(0))
        }
    }

    pub async fn get_latest_done_epoch(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
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
            Ok(Some(
                row.get::<_, i64>("batch_range_end_epoch").to_u64().unwrap(),
            ))
        } else {
            Ok(Some(0))
        }
    }

    pub async fn get_latest_sync_committee_in_progress(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
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
            Ok(Some(helpers::slot_to_sync_committee_id(
                row.get::<_, i64>("slot").to_u64().unwrap(),
            )))
        } else {
            Ok(Some(0))
        }
    }

    pub async fn get_latest_done_sync_committee(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
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
            Ok(Some(helpers::slot_to_sync_committee_id(
                row.get::<_, i64>("slot").to_u64().unwrap(),
            )))
        } else {
            Ok(Some(0))
        }
    }

    pub async fn count_jobs_in_progress(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
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
            Ok(Some(row.get::<_, i64>("count").to_u64().unwrap()))
        } else {
            Ok(Some(0))
        }
    }

    pub async fn get_merkle_paths_for_epoch(
        &self,
        epoch_id: i32,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        // Query all merkle paths for the given epoch_id
        let rows = self
            .client
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

    // pub async fn get_compute_finsihed_jobs_to_proccess_onchain_call(
    //     &self,
    //     last_epoch: JobStatus,
    // ) -> Result<Vec<JobSchema>, Box<dyn std::error::Error + Send + Sync>> {
    //     let rows = self
    //         .client
    //         .query(
    //             "SELECT * FROM jobs
    //              WHERE job_status = 'OFFCHAIN_COMPUTATION_FINISHED' AND job_type = 'EPOCH_BATCH_UPDATE'  AND batch_range_end_epoch <= $1",
    //             &[&last_epoch],
    //         )
    //         .await?;

    //     // Map rows into Job structs
    //     let jobs: Vec<JobSchema> = rows
    //         .into_iter()
    //         .map(|row: Row| JobSchema {
    //             job_uuid: row.get("job_uuid"),
    //             job_status: row.get("job_status"),
    //             slot: row.get("slot"),
    //             batch_range_begin_epoch: row.get("batch_range_begin_epoch"),
    //             batch_range_end_epoch: row.get("batch_range_end_epoch"),
    //             job_type: row.get("type"),
    //             updated_at: row.get("updated_at"),
    //         })
    //         .collect();

    //     Ok(jobs)
    // }

    pub async fn get_jobs_with_status(
        &self,
        desired_status: JobStatus,
    ) -> Result<Vec<JobSchema>, Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn update_job_status(
        &self,
        job_id: Uuid,
        new_status: JobStatus,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn set_failure_info(
        &self,
        job_id: Uuid,
        failed_at_step: JobStatus,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE jobs SET failed_at_step = $1, updated_at = NOW(), last_failure_time = NOW() WHERE job_uuid = $2",
                &[&failed_at_step.to_string(), &job_id],
            )
            .await?;
        Ok(())
    }

    pub async fn count_epoch_jobs_waiting_for_sync_committe_update(
        &self,
        latest_verified_sync_committee: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn set_ready_to_broadcast_for_batch_epochs(
        &self,
        first_epoch: u64,
        last_epoch: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn set_ready_to_broadcast_for_batch_epochs_to(
        &self,
        to_epoch: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn set_ready_to_broadcast_for_sync_committee(
        &self,
        sync_committee_id: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn set_job_txhash(
        &self,
        job_id: Uuid,
        txhash: Felt,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE jobs SET tx_hash = $1, updated_at = NOW() WHERE job_uuid = $2",
                &[&txhash.to_hex_string(), &job_id],
            )
            .await?;
        Ok(())
    }

    // pub async fn cancell_all_unfinished_jobs(
    //     &self,
    // ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //     self.client
    //         .execute(
    //             "UPDATE jobs SET status = $1, updated_at = NOW() WHERE status = 'FETCHING'",
    //             &[&JobStatus::Cancelled.to_string()],
    //         )
    //         .await?;
    //     Ok(())
    // }

    pub async fn insert_merkle_path_for_epoch(
        &self,
        epoch: u64,
        path_index: u64,
        path: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn get_jobs_with_statuses(
        &self,
        desired_statuses: Vec<JobStatus>,
    ) -> Result<Vec<JobSchema>, Box<dyn std::error::Error + Send + Sync>> {
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
    //

    // pub async fn insert_job_log_entry(
    //     &self,
    //     job_id: u64,
    //     event_type: JobLogEntry,
    //     details: String,
    // ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    //     self.client
    //         .execute(
    //             "INSERT INTO job_logs (job_id, event_type, details)
    //          VALUES ($1, $2, $3)",
    //             &[&job_id.to_string(), &event_type.to_string(), &details],
    //         )
    //         .await?;

    //     Ok(())
    // }
    //
    pub async fn update_daemon_state_info(
        &self,
        latest_known_beacon_slot: u64,
        latest_known_beacon_block: FixedBytes<32>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.client
            .execute(
                "UPDATE daemon_state SET latest_known_beacon_slot = $1, latest_known_beacon_block = NOW()",
                &[&latest_known_beacon_slot.to_string(), &latest_known_beacon_block.to_string()],
            )
            .await?;
        Ok(())
    }

    pub async fn count_total_jobs(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let row = self
            .client
            .query_one("SELECT COUNT(*) as count FROM jobs", &[])
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    pub async fn count_successful_jobs(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        let row = self
            .client
            .query_one(
                "SELECT COUNT(*) as count FROM jobs WHERE job_status = 'DONE'",
                &[],
            )
            .await?;

        Ok(row.get::<_, i64>("count").to_u64().unwrap_or(0))
    }

    pub async fn get_average_job_duration(
        &self,
    ) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        let row = self
            .client
            .query_one(
                "SELECT EXTRACT(EPOCH FROM AVG(updated_at - created_at))::INTEGER as avg_duration
                 FROM jobs
                 WHERE job_status = 'DONE'",
                &[],
            )
            .await?;

        Ok(i64::from(
            row.get::<_, Option<i32>>("avg_duration").unwrap_or(0),
        ))
    }

    pub async fn get_recent_batch_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, Box<dyn std::error::Error + Send + Sync>> {
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
                let job = Self::map_row_to_job(row.clone()).unwrap();
                JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                }
            })
            .collect();

        Ok(jobs)
    }

    pub async fn get_recent_sync_committee_jobs(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, Box<dyn std::error::Error + Send + Sync>> {
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
                let job = Self::map_row_to_job(row.clone()).unwrap();
                JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                }
            })
            .collect();

        Ok(jobs)
    }

    pub async fn is_connected(&self) -> bool {
        match self.client.query_one("SELECT 1", &[]).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub async fn get_recent_atlantic_queries_in_progress(
        &self,
        limit: i64,
    ) -> Result<Vec<JobWithTimestamps>, Box<dyn std::error::Error + Send + Sync>> {
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
                let job = Self::map_row_to_job(row.clone()).unwrap();
                JobWithTimestamps {
                    job,
                    created_at: row.get("created_time"),
                    updated_at: row.get("updated_time"),
                    tx_hash: row.get("tx_hash"),
                }
            })
            .collect();

        Ok(jobs)
    }

    pub async fn get_jobs_count_by_status(
        &self,
    ) -> Result<Vec<JobStatusCount>, Box<dyn std::error::Error + Send + Sync>> {
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

            let job_status = JobStatus::from_str(&status_str)
                .map_err(|err| format!("Failed to parse job status from DB row: {}", err))?;

            db_counts.insert(job_status, status_count);
        }

        let all_possible_statuses = vec![
            JobStatus::Created,
            JobStatus::StartedTraceGeneration,
            JobStatus::ProgramInputsPrepared,
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
    fn map_row_to_job(row: Row) -> Result<JobSchema, Box<dyn std::error::Error + Send + Sync>> {
        let job_status_str: String = row.get("job_status");
        let job_status = job_status_str
            .parse::<JobStatus>()
            .map_err(|err| format!("Failed to parse job status: {}", err))?;

        let job_type_str: String = row.get("type");
        let job_type = job_type_str
            .parse::<JobType>()
            .map_err(|err| format!("Failed to parse job type: {}", err))?;

        let failed_at_step: Option<JobStatus> = row
            .get::<_, Option<String>>("failed_at_step")
            .map(|step| {
                step.parse::<JobStatus>()
                    .map_err(|err| format!("Failed to parse job type: {}", err))
            })
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
            atlantic_proof_generate_batch_id: row.get("atlantic_proof_generate_batch_id"),
            atlantic_proof_wrapper_batch_id: row.get("atlantic_proof_wrapper_batch_id"),
            failed_at_step,
            retries_count: row.get("retries_count"),
            last_failure_time,
        })
    }
}

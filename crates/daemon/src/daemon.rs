use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::types::job::JobStatus;
use bankai_core::{db::manager::DatabaseManager, types::job::Job, utils::constants, BankaiClient};
use dotenv::from_filename;
use std::{env, sync::Arc};
use tokio::sync::mpsc;
use tracing::{error, info, Level};

use crate::error::DaemonError;
use crate::job_manager::resume::update_job_status_for_resume;
use crate::job_manager::retry::update_job_status_for_retry;
use crate::job_processor::scheduler::create_new_jobs;

use crate::{beacon_listener::BeaconListener, job_processor::JobProcessor};

pub struct Daemon {
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
    job_processor: JobProcessor,
    beacon_listener: BeaconListener,
    tx: mpsc::Sender<Job>,
    rx: mpsc::Receiver<Job>,
    rx_head_event: mpsc::Receiver<HeadEvent>,
}

impl Daemon {
    pub async fn new() -> Result<Self, DaemonError> {
        // Load .env.sepolia file
        from_filename(".env.sepolia").ok();

        // Replace the current subscriber setup with JSON formatting
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_max_level(Level::INFO)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        // Validate environment variables
        let _ = check_env_vars().map_err(|e| {
            error!("Error: {}", e);
            std::process::exit(1); // Exit if validation fails
        });

        info!("Starting Bankai light-client daemon...");

        // Create channel for job communication
        let (tx, rx): (mpsc::Sender<Job>, mpsc::Receiver<Job>) = mpsc::channel(32);
        let (tx_head_event, rx_head_event): (mpsc::Sender<HeadEvent>, mpsc::Receiver<HeadEvent>) =
            mpsc::channel(32);

        // Initialize database connection
        let connection_string = format!(
            "host={} user={} password={} dbname={}",
            env::var("POSTGRES_HOST").unwrap().as_str(),
            env::var("POSTGRES_USER").unwrap().as_str(),
            env::var("POSTGRES_PASSWORD").unwrap().as_str(),
            env::var("POSTGRES_DB").unwrap().as_str()
        );

        // Create a new DatabaseManager
        let db_manager = Arc::new(DatabaseManager::new(&connection_string).await);
        let bankai = Arc::new(BankaiClient::new(true).await);

        // Initialize components
        let job_processor = JobProcessor::new(db_manager.clone(), bankai.clone());

        let events_endpoint = format!(
            "{}/eth/v1/events?topics=head",
            env::var("BEACON_RPC_URL").unwrap().as_str()
        );
        let beacon_listener = BeaconListener::new(events_endpoint, tx_head_event.clone());

        Ok(Self {
            db_manager,
            bankai,
            job_processor,
            beacon_listener,
            tx,
            rx,
            rx_head_event,
        })
    }

    pub async fn run(&mut self) -> Result<(), DaemonError> {
        // Start beacon listener
        self.beacon_listener.start().await?;

        // when we start, we want to make sure we resume the jobs that were interrupted
        let interupted_jobs = self.db_manager.fetch_interrupted_jobs().await?;
        for job in interupted_jobs {
            update_job_status_for_resume(self.tx.clone(), self.db_manager.clone(), job).await?;
        }

        // Start the job processor and the head event processor
        self.start_job_processor().await;
        self.start_head_event_processor().await;

        Ok(())
    }

    async fn start_job_processor(&mut self) {
        let processor = self.job_processor.clone();
        let rx = std::mem::replace(&mut self.rx, mpsc::channel(32).1);

        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(job) = rx.recv().await {
                let job_id = job.job_id;
                let processor_clone = processor.clone();
                match job.job_status {
                    JobStatus::OffchainProofRequested
                    | JobStatus::WrapProofRequested
                    | JobStatus::OffchainComputationFinished => {
                        tokio::spawn(async move {
                            if let Err(e) = processor_clone.process_proof_job(job.clone()).await {
                                error!(
                                    job_id = %job_id,
                                    job_type = ?job.job_type,
                                    job_status = ?job.job_status,
                                    error = %e,
                                    error_type = ?std::any::type_name_of_val(&e),
                                    "Error processing proof job"
                                );
                                processor_clone
                                    .handle_job_error(job_id)
                                    .await
                                    .map_err(|e| {
                                        error!(
                                            job_id = %job_id,
                                            error = %e,
                                            error_type = ?std::any::type_name_of_val(&e),
                                            "Error handling job error"
                                        );
                                        e
                                    })
                                    .unwrap();
                            }
                        });
                    }
                    _ => {
                        tokio::spawn(async move {
                            if let Err(e) = processor_clone.process_trace_gen_job(job.clone()).await {
                                error!(
                                    job_id = %job_id,
                                    job_type = ?job.job_type,
                                    job_status = ?job.job_status,
                                    error = %e,
                                    error_type = ?std::any::type_name_of_val(&e),
                                    "Error processing trace generation job"
                                );
                                processor_clone
                                    .handle_job_error(job_id)
                                    .await
                                    .map_err(|e| {
                                        error!(
                                            job_id = %job_id,
                                            error = %e,
                                            error_type = ?std::any::type_name_of_val(&e),
                                            "Error handling job error"
                                        );
                                        e
                                    })
                                    .unwrap();
                            }
                        });
                    }
                }
            }
        });
    }

    //Spawn a background task to process jobs
    async fn start_head_event_processor(&mut self) {
        let db_manager = self.db_manager.clone();
        let bankai = self.bankai.clone();
        let tx = self.tx.clone();
        let rx_head_event = std::mem::replace(&mut self.rx_head_event, mpsc::channel(32).1);
        tokio::spawn(async move {
            let mut rx_head_event = rx_head_event;
            while let Some(event) = rx_head_event.recv().await {
                // Priorities retries
                let retryable_jobs = db_manager
                    .fetch_retryable_jobs(constants::MAX_JOB_RETRIES_COUNT as i64)
                    .await
                    .unwrap();
                for job in retryable_jobs {
                    // we only update the job status here, the job will be requeued in the next iteration
                    update_job_status_for_retry(
                        tx.clone(),
                        db_manager.clone(),
                        bankai.clone(),
                        job.clone(),
                    )
                    .await
                    .map_err(|e| {
                        error!(
                            job_id = %job.job_id,
                            job_type = ?job.job_type,
                            job_status = ?job.job_status,
                            error = %e,
                            error_type = ?std::any::type_name_of_val(&e),
                            "Error updating job status for retry"
                        );
                        e
                    })
                    .unwrap();
                }

                // Every 5 slots, requeue the jobs that are currently proving
                if event.slot % 5 == 0 {
                    let jobs = db_manager.fetch_jobs_in_proof_generation().await.unwrap();
                    for job in jobs {
                        let _ = tx.send(job).await;
                    }

                    let jobs = db_manager.fetch_jobs_waiting_for_broadcast().await.unwrap();
                    for job in jobs {
                        let _ = tx.send(job).await;
                    }
                }
                let db_clone = db_manager.clone();
                let bankai_clone = bankai.clone();
                let tx_clone = tx.clone();

                // Process synchronously in the main event loop
                let _ = db_clone
                    .update_daemon_state_info(event.slot, event.block)
                    .await;

                create_new_jobs(
                    &event,
                    db_clone.clone(),
                    bankai_clone.clone(),
                    tx_clone.clone(),
                )
                .await
                .map_err(|e| {
                    error!(
                        event_slot = event.slot,
                        event_block = %event.block,
                        error = %e,
                        error_type = ?std::any::type_name_of_val(&e),
                        "Error creating new jobs"
                    );
                    e
                })
                .unwrap();
            }
        });
    }
}

// Checking status of env vars
pub fn check_env_vars() -> Result<(), String> {
    let required_vars = [
        "BEACON_RPC_URL",
        "STARKNET_RPC_URL",
        "STARKNET_ADDRESS",
        "STARKNET_PRIVATE_KEY",
        "ATLANTIC_API_KEY",
        "PROOF_REGISTRY",
        "POSTGRES_HOST",
        "POSTGRES_USER",
        "POSTGRES_PASSWORD",
        "POSTGRES_DB",
        "TRANSACTOR_API_KEY",
    ];

    for &var in &required_vars {
        if env::var(var).is_err() {
            return Err(format!("Environment variable `{}` is not set", var));
        }
    }

    Ok(())
}

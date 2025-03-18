use alloy_rpc_types_beacon::events::HeadEvent;
use bankai_core::types::job::JobStatus;
use bankai_core::{
    db::manager::DatabaseManager,
    BankaiClient,
    types::job::Job,
    utils::constants,
    utils::config::BankaiConfig,
    clients::starknet::StarknetClient,
};
use dotenv::from_filename;
use std::{env, sync::Arc};
use tokio::sync::mpsc;
use tokio::signal;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;
use axum::Router;
use std::net::SocketAddr;

use crate::error::DaemonError;
use crate::job_processor::scheduler::create_new_jobs;

use crate::{beacon_listener::BeaconListener, job_manager::JobManager, job_processor::JobProcessor};

pub struct Daemon {
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
    job_processor: JobProcessor,
    job_manager: JobManager,
    beacon_listener: Option<BeaconListener>,
    tx: mpsc::Sender<Job>,
    rx: mpsc::Receiver<Job>,
    rx_head_event: mpsc::Receiver<HeadEvent>,
}

impl Daemon {
    pub async fn new() -> Result<Self, DaemonError> {
        // Load .env.sepolia file
        from_filename(".env.sepolia").ok();

        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::INFO)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        // // Validate environment variables
        // let _ = check_env_vars().map_err(|e| {
        //     error!("Error: {}", e);
        //     std::process::exit(1); // Exit if validation fails
        // });

        info!("Starting Bankai light-client daemon...");

        // Create channel for job communication
        let (tx, rx): (mpsc::Sender<Job>, mpsc::Receiver<Job>) = mpsc::channel(32);
        let (tx_head_event, rx_head_event): (mpsc::Sender<HeadEvent>, mpsc::Receiver<HeadEvent>) = mpsc::channel(32);
        
        // Initialize database connection
        let connection_string = format!(
            "host={} user={} password={} dbname={}",
            env::var("POSTGRESQL_HOST").unwrap().as_str(),
            env::var("POSTGRESQL_USER").unwrap().as_str(),
            env::var("POSTGRESQL_PASSWORD").unwrap().as_str(),
            env::var("POSTGRESQL_DB_NAME").unwrap().as_str()
        );

        // Create a new DatabaseManager
        let db_manager = Arc::new(DatabaseManager::new(&connection_string).await);
        let bankai = Arc::new(BankaiClient::new().await);

        // Initialize components
        let job_processor = JobProcessor::new(db_manager.clone(), bankai.clone());
        let job_manager = JobManager::new(db_manager.clone(), bankai.clone(), tx.clone());
        // let broadcaster = OnchainBroadcaster::new(db_manager.clone(), bankai.clone());
        
        // Initialize beacon listener if enabled
        let beacon_listener = if constants::BEACON_CHAIN_LISTENER_ENABLED {
            let events_endpoint = format!(
                "{}/eth/v1/events?topics=head",
                env::var("BEACON_RPC_URL").unwrap().as_str()
            );
            
            Some(BeaconListener::new(
                events_endpoint,
                tx_head_event.clone(),
            ))
        } else {
            None
        };

        Ok(Self {
            db_manager,
            bankai,
            job_processor,
            job_manager,
            beacon_listener,
            tx,
            rx,
            rx_head_event,
        })
    }

    pub async fn run(&mut self) -> Result<(), DaemonError> {
        // Start heartbeat task
        // self.start_heartbeat().await;
        
        // Start job processing task
        // self.start_job_processor().await;
        
        // // Start API server
        // let server_handle = self.start_api_server().await?;
        
        // // Resume unfinished jobs if enabled
        // if constants::JOBS_RESUME_ENABLED {
        //     self.job_manager.resume_unfinished_jobs().await?;
        // }
        
        // // Retry failed jobs if enabled
        // if constants::JOBS_RETRY_ENABLED {
        //     self.job_manager.retry_failed_jobs(false).await?;
        // }
        
        // Start beacon listener if enabled
        if let Some(listener) = &self.beacon_listener {
            listener.start().await?;
        }
        
        // Start the job processor and the head event processor
        self.start_job_processor().await;
        self.start_head_event_processor().await;
        
        // Start job retry watcher
        // self.start_job_retry_watcher().await;
        
        // Wait for the server task to finish
        // server_handle.await?;
        
        Ok(())
    }

    async fn start_heartbeat(&self) {
        tokio::spawn(async move {
            loop {
                info!("[HEARTBEAT] Daemon is alive");
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            }
        });
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
                    JobStatus::OffchainProofRequested | JobStatus::WrapProofRequested => {
                        tokio::spawn(async move {
                            if let Err(e) = processor_clone.process_proof_job(job.clone()).await {
                                error!("Error processing job {}: {}", job_id, e);
                                processor_clone.handle_job_error(job_id).await;
                            }
                        });
                    }
                    _ => {
                        tokio::spawn(async move {
                            if let Err(e) = processor_clone.process_trace_gen_job(job.clone()).await {
                                error!("Error processing job {}: {}", job_id, e);
                                processor_clone.handle_job_error(job_id).await;
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

                // Every 5 slots, requeue the jobs that are currently proving
                if event.slot % 5 == 0 {
                    let jobs = db_manager.fetch_jobs_in_proof_generation().await.unwrap();
                    for job in jobs {
                        let _ = tx.send(job).await;
                    }
                    
                }
                let db_clone = db_manager.clone();
                let bankai_clone = bankai.clone();
                let tx_clone = tx.clone();
                
                // Inner spawn - Individual head event processor
                tokio::spawn(async move {
                    let _ = db_clone
                        .update_daemon_state_info(event.slot, event.block)
                        .await;

                    println!("Processing head event for slot {}, block {}", event.slot, event.block);
                    create_new_jobs(&event, db_clone, bankai_clone, tx_clone).await;

                });
            }
        });
    }

    // async fn start_job_retry_watcher(&self) {
    //     let db_manager = self.db_manager.clone();
    //     let tx = self.tx.clone();
        
    //     tokio::spawn(async move {
    //         let retrier = JobRetrier::new(db_manager, tx);
            
    //         loop {
    //             tokio::time::sleep(std::time::Duration::from_secs(
    //                 constants::JOBS_RETRY_CHECK_INTERVAL,
    //             )).await;
                
    //             let _ = retrier.retry_failed_jobs(true).await;
    //         }
    //     });
    // }

   
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Gracefully shutting down...");
        },
        _ = terminate => {
            info!("Gracefully shutting down...");
        },
    }
}

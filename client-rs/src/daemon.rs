#![allow(dead_code)]
#![allow(unused_imports)]
mod bankai_client;
mod config;
mod constants;
mod contract_init;
pub mod epoch_batch;
mod epoch_update;
mod execution_header;
mod helpers;
mod routes;
mod state;
mod sync_committee;
mod traits;
mod utils;
//use alloy_primitives::TxHash;
//use alloy_primitives::FixedBytes;
use alloy_rpc_types_beacon::events::HeadEvent;
use axum::{
    extract::DefaultBodyLimit,
    //http::{header, StatusCode},
    routing::get,
    Router,
};
use bankai_client::BankaiClient;
use config::BankaiConfig;
//use constants::SLOTS_PER_EPOCH;
use dotenv::from_filename;
use helpers::{
    get_first_epoch_for_sync_committee, get_first_slot_for_epoch, get_last_epoch_for_sync_committee,
};
use num_traits::cast::ToPrimitive;
use reqwest;
use routes::dashboard::handle_get_dashboard;
use starknet::core::types::Felt;
use state::check_env_vars;
use state::{AppState, Job};
use state::{AtlanticJobType, Error, JobStatus, JobType};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::{signal, task};
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::{timeout::TimeoutLayer, trace::TraceLayer};
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use traits::Provable;
use utils::{cairo_runner::CairoRunner, database_manager::DatabaseManager};
//use std::error::Error as StdError;
use epoch_batch::EpochUpdateBatch;
use routes::{
    handle_get_committee_hash,
    handle_get_epoch_proof, // handle_get_epoch_update,
    handle_get_job_status,
    handle_get_latest_verified_committee,
    handle_get_latest_verified_slot,
    handle_get_merkle_paths_for_epoch,
    handle_get_status,
    handle_root_route,
};
use std::net::SocketAddr;
use sync_committee::SyncCommitteeUpdate;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env.sepolia file
    from_filename(".env.sepolia").ok();

    let slot_listener_toggle = constants::BEACON_CHAIN_LISTENER_ENABLED;

    let subscriber = FmtSubscriber::builder()
        //.with_max_level(Level::DEBUG)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Validate environment variables
    let _ = check_env_vars().map_err(|e| {
        error!("Error: {}", e);
        std::process::exit(1); // Exit if validation fails
    });

    info!("Starting Bankai light-client daemon...");

    let (tx, mut rx): (mpsc::Sender<Job>, mpsc::Receiver<Job>) = mpsc::channel(32);

    //let (tx, mut rx) = mpsc::channel(32);

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

    // Beacon node endpoint construction for events
    let events_endpoint = format!(
        "{}/eth/v1/events?topics=head",
        env::var("BEACON_RPC_URL").unwrap().as_str()
    );

    //let events_endpoint = format!("{}/eth/v1/events?topics=head", beacon_node_url)
    let db_manager_for_listener = db_manager.clone();
    let db_manager_for_watcher = db_manager.clone();
    let bankai_for_listener = bankai.clone();

    let tx_for_listener = tx.clone();
    let tx_for_watcher = tx.clone();

    let app_state: AppState = AppState {
        db_manager: db_manager.clone(),
        tx,
        bankai: bankai.clone(),
    };

    tokio::spawn(async move {
        loop {
            info!("[HEARTBEAT] Daemon is alive");
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    //Spawn a background task to process jobs
    tokio::spawn(async move {
        while let Some(job) = rx.recv().await {
            let job_id = job.job_id;
            let db_clone = db_manager.clone();
            let bankai_clone = Arc::clone(&bankai);

            // Spawn a *new task* for each job — now they can run in parallel
            tokio::spawn(async move {
                match process_job(job, db_clone.clone(), bankai_clone.clone()).await {
                    Ok(_) => {
                        info!("Job {} completed successfully", job_id);
                    }
                    Err(e) => {
                        let job_data = db_clone.get_job_by_id(job_id).await.unwrap().unwrap();
                        let _ = db_clone.set_failure_info(job_id, job_data.job_status).await;
                        let _ = db_clone.update_job_status(job_id, JobStatus::Error).await;
                        error!("Error processing job {}: {}", job_id, e);
                    }
                }
            });
        }
    });

    let app = Router::new()
        .route("/", get(handle_root_route))
        .route("/status", get(handle_get_status))
        .route(
            "/get_verified_epoch_proof/:epoch",
            get(handle_get_epoch_proof),
        )
        .route(
            "/get_verified_committee_hash/:committee_id",
            get(handle_get_committee_hash),
        )
        .route(
            "/get_merkle_paths_for_epoch/:epoch_id",
            get(handle_get_merkle_paths_for_epoch),
        )
        // .route(
        //     "/debug/get_epoch_update/:slot",
        //     get(handle_get_epoch_update),
        // )
        .route(
            "/debug/get_latest_verified_epoch",
            get(handle_get_latest_verified_slot),
        )
        .route(
            "/debug/get_latest_verified_committee",
            get(handle_get_latest_verified_committee),
        )
        .route("/debug/get_job_status", get(handle_get_job_status))
        // Add dashboard route here
        .route("/dashboard", get(handle_get_dashboard))
        // .route("/get-merkle-inclusion-proof", get(handle_get_merkle_inclusion_proof))
        .layer(DefaultBodyLimit::disable())
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http()), // Example: for logging/tracing
        )
        .layer((
            // Graceful shutdown will wait for outstanding requests to complete
            // Because of this timeourt setting, requests don't hang forever
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .with_state(app_state);

    let addr = "0.0.0.0:3001".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Bankai RPC HTTP server is listening on http://{}", addr);

    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap();
    });

    // After RPC init, we do some startup checks before start listening to beacon chain:
    //
    // 🔄 Resume any unfinished jobs before processing new ones
    if constants::JOBS_RESUME_ENABLED {
        resume_unfinished_jobs(db_manager_for_listener.clone(), tx_for_listener.clone()).await?;
    }

    // Retry any failed jobs before processing new ones
    if constants::JOBS_RETRY_ENABLED {
        retry_failed_jobs(db_manager_for_listener.clone(), tx_for_listener.clone()).await?;
    }

    //enqueue_sync_committee_jobs();
    //enqueue_batch_epochs_jobs();
    //

    // Listen for the new slots on BeaconChain
    // Create an HTTP client
    let http_stream_client = reqwest::Client::new();
    if slot_listener_toggle {
        tokio::spawn(async move {
            loop {
                // Send the request to the Beacon node
                let response = match http_stream_client
                    .get(&events_endpoint)
                    //.timeout(std::time::Duration::from_secs(30)) - cannot do this because this will give timeout after each duration since we not using HTTP Pooling here but HTTP streaming
                    .send()
                    .await
                {
                    Ok(r) => r,
                    Err(e) => {
                        error!("Failed to connect: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue; // retry
                    }
                };

                if !response.status().is_success() {
                    error!("Got non-200: {}", response.status());
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue; // retry
                }

                info!("Listening for new slots, epochs and sync committee updates...");

                let mut stream = response.bytes_stream();

                loop {
                    match timeout(Duration::from_secs(30), stream.next()).await {
                        // Timed out; handle it locally
                        Err(_elapsed) => {
                            warn!(
                            "Timed out waiting for new slot beacon chain event chunk. Maybe some slots was skipped. Will reconnect..."
                        );
                            break;
                        }
                        Ok(Some(Ok(bytes))) => {
                            if let Ok(event_text) = String::from_utf8(bytes.to_vec()) {
                                // Preprocess the event text
                                if let Some(json_data) =
                                    helpers::extract_json_from_event(&event_text)
                                {
                                    match serde_json::from_str::<HeadEvent>(&json_data) {
                                        Ok(parsed_event) => {
                                            let epoch_id =
                                                helpers::slot_to_epoch_id(parsed_event.slot);
                                            let sync_committee_id =
                                                helpers::slot_to_sync_committee_id(
                                                    parsed_event.slot,
                                                );
                                            info!(
                                                "[EVENT] New beacon slot detected: {} |  Block: {} | Epoch: {} | Sync committee: {} | Is epoch transition: {}",
                                                parsed_event.slot, parsed_event.block, epoch_id, sync_committee_id, parsed_event.epoch_transition
                                            );

                                            handle_beacon_chain_head_event(
                                                parsed_event,
                                                bankai_for_listener.clone(),
                                                db_manager_for_listener.clone(),
                                                tx_for_listener.clone(),
                                            )
                                            .await;
                                        }
                                        Err(err) => {
                                            warn!("Failed to parse JSON data: {}", err);
                                        }
                                    }
                                } else {
                                    warn!("No valid JSON data found in event: {}", event_text);
                                }
                            }
                        }
                        Ok(Some(Err(e))) => {
                            warn!("Beacon chain client stream error: {}", e);
                            break; // break the while, then reconnect
                        }
                        Ok(None) => {
                            warn!("Beacon chain client stream ended");
                            // Stream ended
                            break;
                        }
                    }
                }
                // If we got here because of `timeout` returning `Err(_)`, that means 30s
                // passed without a single chunk of data arriving or
                // the RPC server has closed connection or some other unknown network error occured

                // If we exit the while, we reconnect in the outer loop
                info!("Timeout waiting for next event, reconnecting to beacon node...");
            }
        });
    }

    // Run check and retry failed jobs periodicially
    tokio::spawn(async move {
        loop {
            retry_failed_jobs(db_manager_for_watcher.clone(), tx_for_watcher.clone()).await;
            tokio::time::sleep(std::time::Duration::from_secs(
                constants::JOBS_RETRY_CHECK_INTERVAL,
            ))
            .await;
        }
    });

    // Wait for the server task to finish
    server_task.await?;

    Ok(())
}

async fn handle_beacon_chain_head_event(
    parsed_event: HeadEvent,
    bankai: Arc<BankaiClient>,
    db_manager: Arc<DatabaseManager>,
    tx: mpsc::Sender<Job>,
) -> () {
    let current_epoch_id = helpers::slot_to_epoch_id(parsed_event.slot);
    let current_sync_committee_id = helpers::slot_to_sync_committee_id(parsed_event.slot);

    if parsed_event.epoch_transition {
        //info!("Beacon Chain epoch transition detected. New epoch: {} | Starting processing epoch proving...", epoch_id);
        info!(
            "Beacon Chain epoch transition detected. New epoch: {}",
            current_epoch_id
        );

        // Check also now if slot is the moment of switch to new sync committee set
        if parsed_event.slot % constants::SLOTS_PER_SYNC_COMMITTEE == 0 {
            info!(
                "Beacon Chain sync committee rotation occured. Slot {} | Sync committee id: {}",
                parsed_event.slot, current_sync_committee_id
            );
        }
    }

    let _ = db_manager
        .update_daemon_state_info(parsed_event.slot, parsed_event.block)
        .await;

    // We can do all circuit computations up to latest slot in advance, but the onchain broadcasts must be send in correct order
    // By correct order mean that within the same sync committe the epochs are not needed to be broadcasted in order
    // but the order of sync_commite_update->epoch_update must be correct, we firstly need to have correct sync committe veryfied
    // before we verify epoch "belonging" to this sync committee

    let latest_verified_epoch_slot = bankai
        .starknet_client
        .get_latest_epoch_slot(&bankai.config)
        .await
        .unwrap()
        .to_u64()
        .unwrap();

    let latest_verified_sync_committee_id = bankai
        .starknet_client
        .get_latest_committee_id(&bankai.config)
        .await
        .unwrap()
        .to_u64()
        .unwrap();

    let lowest_required_committee_update_slot =
        (latest_verified_sync_committee_id) * constants::SLOTS_PER_SYNC_COMMITTEE;

    let latest_verified_epoch_id = helpers::slot_to_epoch_id(latest_verified_epoch_slot);
    let epochs_behind = current_epoch_id - latest_verified_epoch_id;

    let _ = evaluate_jobs_statuses(
        db_manager.clone(),
        latest_verified_sync_committee_id,
        latest_verified_epoch_slot,
    )
    .await
    .map_err(|e| {
        error!("Error evaluating jobs statuses: {}", e);
    });
    let _ = broadcast_onchain_ready_jobs(db_manager.clone(), bankai.clone())
        .await
        .map_err(|e| {
            error!("Error executing broadcast onchain ready jobs: {}", e);
        });

    // We getting the last slot in progress to determine next slots to prove
    //let mut last_slot_in_progress: u64 = 0;
    // /let mut last_epoch_in_progress: u64 = 0;
    // let mut last_sync_committee_in_progress: u64 = 0;
    //

    let last_epoch_in_progress = db_manager
        .get_latest_epoch_in_progress()
        .await
        .unwrap()
        .unwrap();

    let last_done_epoch = db_manager.get_latest_done_epoch().await.unwrap().unwrap();

    let last_sync_committee_in_progress = db_manager
        .get_latest_sync_committee_in_progress()
        .await
        .unwrap()
        .unwrap();

    let last_done_sync_committee = db_manager
        .get_latest_done_sync_committee()
        .await
        .unwrap()
        .unwrap();

    let mut latest_scheduled_epoch = last_epoch_in_progress;
    let mut latest_scheduled_sync_committee = last_sync_committee_in_progress;

    if latest_verified_epoch_id > last_epoch_in_progress {
        if last_epoch_in_progress == 0 {
            //info!("Starting daemon on clean epochs jobs table");
        } else {
            warn!(
                "Something may be wrong, last verified epoch is greather than last epoch in progress"
            );
        }
        // So we should schedule the greater epoch, which is
        latest_scheduled_epoch = latest_verified_epoch_id;
    }

    if latest_verified_sync_committee_id > last_sync_committee_in_progress {
        if last_sync_committee_in_progress == 0 {
            //info!("Starting daemon on clean sync committees jobs table");
        } else {
            warn!(
                "Something may be wrong, last verified sync committee is greather than last sync committee in progress"
            );
        }

        latest_scheduled_sync_committee = latest_verified_sync_committee_id;
    }

    info!(
        "Current state: Beacon Chain: [Slot: {} Epoch: {} Sync Committee: {}] | Latest verified: [Slot: {} Epoch: {} Sync Committee: {}] | Latest in progress: [Epoch: {} Sync Committee: {}] | Latest done: [Epoch: {} Sync Committee: {}] | Sync in progress...",
        parsed_event.slot, current_epoch_id, current_sync_committee_id, latest_verified_epoch_slot, latest_verified_epoch_id, latest_verified_sync_committee_id, last_epoch_in_progress, last_sync_committee_in_progress, last_done_epoch, last_done_sync_committee
    );

    // Decide basing on actual state
    // if helpers::get_sync_committee_id_by_epoch(latest_scheduled_epoch + 1)
    //     > latest_scheduled_sync_committee
    // {
    //     // We reached end of current sync committee, need to schedule new sync committee proving
    //     match run_sync_committee_update_job(
    //         db_manager.clone(),
    //         latest_scheduled_sync_committee + 1,
    //         tx.clone(),
    //     )
    //     .await
    //     {
    //         Ok(()) => {}
    //         Err(e) => {
    //             error!("Error while creating sync committee update job: {}", e);
    //         }
    //     };
    // }
    //

    if !(latest_verified_epoch_slot < lowest_required_committee_update_slot) {
        info!(
            "Lowest required committee update slot: {}",
            lowest_required_committee_update_slot
        );
        if last_sync_committee_in_progress < latest_scheduled_sync_committee {
            if last_done_sync_committee < latest_scheduled_sync_committee {
                // This last check because the delay of data from sequencer update after verification onchain
                match run_sync_committee_update_job(
                    db_manager.clone(),
                    latest_verified_epoch_slot,
                    tx.clone(),
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Error while creating sync committee update job: {}", e);
                    }
                };
            }
        }
    }

    let current_sync_committee_epochs_left =
        helpers::get_last_epoch_for_sync_committee(current_sync_committee_id) - current_epoch_id;
    info!(
        "{} epochs left in current beacon chain sync committee",
        current_sync_committee_epochs_left
    );

    // Decide basing on actual state
    if epochs_behind > constants::TARGET_BATCH_SIZE {
        // is_node_in_sync = true;

        warn!(
            "Bankai is out of sync now. Node is {} epochs behind network. | Sync in progress...",
            epochs_behind
        );

        // Check if we have in progress all epochs that need to be processed, if no, run job
        if latest_scheduled_epoch < (current_epoch_id - constants::TARGET_BATCH_SIZE) {
            // And chceck how many jobs are already in progress and if we fit in the limit
            let in_progress_jobs_count = db_manager.count_jobs_in_progress().await.unwrap();
            if in_progress_jobs_count.unwrap() >= constants::MAX_CONCURRENT_JOBS_IN_PROGRESS {
                info!(
                    "Currently not starting new batch epoch job, MAX_CONCURRENT_JOBS_IN_PROGRESS limit reached, jobs in progress: {}",
                    in_progress_jobs_count.unwrap()
                );
                return;
            }

            let epoch_to_start_from = latest_scheduled_epoch + 1;
            let mut epoch_to_end_on = latest_scheduled_epoch + constants::TARGET_BATCH_SIZE; // To create batch with size of constants::TARGET_BATCH_SIZE epochs

            let currently_processed_sync_committee_id =
                helpers::get_sync_committee_id_by_epoch(epoch_to_start_from);

            info!(
                "Currently processed sync committee epochs ranges from {} to {}. Next sync committee epochs ranges: {} to {}",
                helpers::get_first_epoch_for_sync_committee(currently_processed_sync_committee_id),
                helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id),
                helpers::get_first_epoch_for_sync_committee(currently_processed_sync_committee_id + 1),
                helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id + 1)
            );

            if helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id)
                == epoch_to_start_from
            {}

            // Edge cases handling //
            // Handle the edge case where there is only one epoch in batch left to proccess and this epoch is last epoch in sync committee, if we follow the betch size of 32 always, this souldnt happen:
            if epoch_to_start_from
                == helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id)
            {
                warn!("edge case: only one epoch left to proccess in batch in this sync committee");
                epoch_to_end_on = epoch_to_start_from;
            }
            // Same, if we follow the betch size of 32 always, this souldnt happen, but if we have not same size batches, it can be trigerred also:
            else if epoch_to_end_on
                > helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id)
            {
                warn!("edge case: batch end epoch {} overlaps with the next sync committee, truncating to the last epoch: {} of corresponding sync committee: {}",
                    epoch_to_end_on, helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id), currently_processed_sync_committee_id);
                // The end epoch is further that current sync committee
                // In this case we can simply assingn sync commite latest epoch as epoch_to_end_on
                epoch_to_end_on = helpers::get_last_epoch_for_sync_committee(
                    currently_processed_sync_committee_id,
                );
            }
            //
            // info!(
            //     "{} epochs left to proccess in associated sync committee term",
            //     helpers::get_last_epoch_for_sync_committee(currently_processed_sync_committee_id)
            //         - latest_scheduled_epoch
            // );
            //
            // Mitigate the issue when Starknet Sequencer RPC responds about last verified slot with delay
            if last_done_epoch < epoch_to_start_from {
                match run_batch_epoch_update_job(
                    db_manager.clone(),
                    get_first_slot_for_epoch(epoch_to_start_from)
                        + (constants::SLOTS_PER_EPOCH * constants::TARGET_BATCH_SIZE),
                    epoch_to_start_from,
                    epoch_to_end_on,
                    tx.clone(),
                )
                .await
                {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Error while creating job: {}", e);
                    }
                };
            }
        } else {
            debug!("All reqired jobs are now queued and processing");
        }
    } else if epochs_behind == constants::TARGET_BATCH_SIZE {
        if last_epoch_in_progress < current_epoch_id {
            // This is when we are synced properly and new epoch batch needs to be inserted
            info!(
                "Target batch size reached. Starting processing next epoch batch. Current Beacon Chain epoch: {} Latest verified epoch: {}",
                current_epoch_id, latest_verified_epoch_id
            );

            let epoch_to_start_from = latest_scheduled_epoch + 1;
            let epoch_to_end_on = latest_scheduled_epoch + constants::TARGET_BATCH_SIZE;
            match run_batch_epoch_update_job(
                db_manager.clone(),
                get_first_slot_for_epoch(epoch_to_start_from)
                    + (constants::SLOTS_PER_EPOCH * constants::TARGET_BATCH_SIZE),
                epoch_to_start_from,
                epoch_to_end_on,
                tx.clone(),
            )
            .await
            {
                Ok(()) => {}
                Err(e) => {
                    error!("Error while creating job: {}", e);
                }
            };
        }
    } else if epochs_behind < constants::TARGET_BATCH_SIZE {
        // When we are in sync and not yet reached the TARGET_BATCH_SIZE epochs lagging behind actual beacon chian state
        let eppchs_left = constants::TARGET_BATCH_SIZE - epochs_behind;
        info!("Target batch size not reached yet, daemon is in sync, {} epochs left to start new batch job", eppchs_left);
    }

    // Check if sync committee update is needed

    if latest_verified_epoch_slot % constants::SLOTS_PER_SYNC_COMMITTEE == 0 {}

    // When we doing EpochBatchUpdate the slot is latest_batch_output
    // So for each batch update we takin into account effectiviely the latest slot from given batch

    //let db_client = db_client.clone();
}

// // This function will enqueue sync committee jobs in database with status CREATED up to the latest sync committee
// async fn enqueue_sync_committee_jobs(
//     db_manager: Arc<DatabaseManager>,
//     bankai: Arc<BankaiClient>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
// }

// // This function will enqueue epoch batch update jobs in database with status CREATED up to the latest able to prove epoch batch
// async fn enqueue_batch_epochs_jobs(
//     db_manager: Arc<DatabaseManager>,
//     bankai: Arc<BankaiClient>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
// }

async fn run_batch_epoch_update_job(
    db_manager: Arc<DatabaseManager>,
    slot: u64,
    batch_range_begin_epoch: u64,
    batch_range_end_epoch: u64,
    tx: mpsc::Sender<Job>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let job_id = Uuid::new_v4();
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::EpochBatchUpdate,
        job_status: JobStatus::Created,
        slot: Some(slot),
        batch_range_begin_epoch: Some(batch_range_begin_epoch),
        batch_range_end_epoch: Some(batch_range_end_epoch),
    };

    // Check to ensure if both epochs belongs to same sync committee
    if helpers::get_sync_committee_id_by_epoch(batch_range_begin_epoch)
        != helpers::get_sync_committee_id_by_epoch(batch_range_end_epoch)
    {
        return Err(
            "Batch range start epoch belongs to different committee than batch range end epoch"
                .into(),
        );
    }

    match db_manager.create_job(job.clone()).await {
        // Insert new job record to DB
        Ok(()) => {
            // Handle success
            info!(
                "[EPOCH BATCH UPDATE] Job created successfully with ID: {} Epochs range from {} to {} | Sync committee involved: {}",
                job_id, batch_range_begin_epoch, batch_range_end_epoch, helpers::get_sync_committee_id_by_epoch(batch_range_end_epoch)
            );
            if tx.send(job).await.is_err() {
                return Err("Failed to send job".into());
            }
            // If starting committee update job, first ensule that the corresponding slot is registered in contract
            Ok(())
        }
        Err(e) => {
            // Handle the error
            return Err(e.into());
        }
    }
}

async fn run_sync_committee_update_job(
    db_manager: Arc<DatabaseManager>,
    //sync_committee_id: u64,
    slot: u64,
    tx: mpsc::Sender<Job>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let job_id = Uuid::new_v4();
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::SyncCommitteeUpdate,
        job_status: JobStatus::Created,
        slot: Some(slot),
        // : Some(helpers::get_first_slot_for_sync_committee(
        //     sync_committee_id,
        // )),
        batch_range_begin_epoch: None,
        batch_range_end_epoch: None,
    };

    match db_manager.create_job(job.clone()).await {
        // Insert new job record to DB
        Ok(()) => {
            // Handle success
            info!(
                "[SYHC COMMITTEE UPDATE] Job created successfully with ID: {}",
                job_id
            );
            if tx.send(job).await.is_err() {
                return Err("Failed to send job".into());
            }
            // If starting committee update job, first ensure that the corresponding slot is registered in contract
            Ok(())
        }
        Err(e) => {
            // Handle the error
            return Err(e.into());
        }
    }
}

async fn evaluate_jobs_statuses(
    db_manager: Arc<DatabaseManager>,
    latest_verified_sync_committee_id: u64,
    latest_verified_epoch_slot: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // The purpose of this function is to manage the sequential nature of onchain verification of epochs and sync committees
    // Firstly we get all jobs with status OFFCHAIN_COMPUTATION_FINISHED
    // We calculating the start and end epoch for provided last verified sync committe
    // and setting READY_TO_BROADCAST status for epochs up to the last epoch belonging to provided latest_verified_sync_committee_id
    let first_epoch = get_first_epoch_for_sync_committee(latest_verified_sync_committee_id + 1);
    let last_epoch = get_last_epoch_for_sync_committee(latest_verified_sync_committee_id + 1);

    //let first_epoch = first_epoch - 32; // So we also broadcast first epoch from next sync committee

    info!(
        "Evaluating jobs for epochs range from {} to {}, for sync committee {}",
        first_epoch, last_epoch, latest_verified_sync_committee_id
    );

    db_manager
        .set_ready_to_broadcast_for_batch_epochs_to(last_epoch) // Set READY_TO_BROADCAST when OFFCHAIN_COMPUTATION_FINISHED
        .await?;

    // db_manager
    //     .set_ready_to_broadcast_for_batch_epochs(first_epoch, last_epoch) // Set READY_TO_BROADCAST when OFFCHAIN_COMPUTATION_FINISHED
    //     .await?;

    db_manager
        .set_ready_to_broadcast_for_sync_committee(latest_verified_sync_committee_id)
        .await?;

    Ok(())
}

async fn resume_unfinished_jobs(
    db_manager: Arc<DatabaseManager>,
    tx: mpsc::Sender<Job>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Checking for unfinished jobs at daemon start...");

    // Fetch jobs that were in progress before shutdown
    let unfinished_jobs = db_manager
        .get_jobs_with_statuses(vec![
            JobStatus::Created,
            JobStatus::ProgramInputsPrepared,
            JobStatus::StartedTraceGeneration,
            JobStatus::PieGenerated,
            JobStatus::AtlanticProofRequested,
            JobStatus::AtlanticProofRetrieved,
            JobStatus::WrapProofRequested,
            JobStatus::WrappedProofDone,
        ])
        .await?;

    if unfinished_jobs.is_empty() {
        info!("No unfinished jobs found.");
        return Ok(());
    }

    info!(
        "Found {} unfinished jobs. Resuming processing...",
        unfinished_jobs.len()
    );

    for job in unfinished_jobs {
        let job_id = job.job_uuid;
        let job_to_resume = Job {
            job_id,
            job_type: job.job_type,
            job_status: job.job_status.clone(),
            slot: Some(job.slot.to_u64().unwrap()),
            batch_range_begin_epoch: job.batch_range_begin_epoch.to_u64(),
            batch_range_end_epoch: job.batch_range_end_epoch.to_u64(),
        };

        let resumed_from_step = job.job_status.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            match job_to_resume.job_type {
                JobType::SyncCommitteeUpdate => {
                    info!(
                        "Resuming job {} from step {}... (sync committee update job for sync committee {})",
                        job_id,
                        resumed_from_step.to_string(),
                        helpers::slot_to_sync_committee_id(job.slot.to_u64().unwrap())
                    );
                }
                JobType::EpochBatchUpdate => {
                    info!(
                        "Resuming job {} from step {}... (batch epoch update job for epochs from {} to {})",
                        job_id, resumed_from_step.to_string(), job.batch_range_begin_epoch, job.batch_range_end_epoch
                    );
                }
            }

            if tx_clone.send(job_to_resume).await.is_err() {
                // return Err("Failed to send job".into());
                error!("Error resuming job: {}", job_id);
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn retry_failed_jobs(
    db_manager: Arc<DatabaseManager>,
    tx: mpsc::Sender<Job>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Checking for failed jobs at daemon start...");

    // Fetch failed jobs
    let errored_jobs = db_manager
        .get_jobs_with_statuses(vec![JobStatus::Error])
        .await?;

    if errored_jobs.is_empty() {
        info!("No failed jobs found.");
        return Ok(());
    }

    warn!(
        "Found {} failed jobs. Trying to retry these jobs...",
        errored_jobs.len()
    );

    for job in errored_jobs {
        let job_id = job.job_uuid;
        let job_to_retry = Job {
            job_id,
            job_type: job.job_type,
            job_status: JobStatus::Created,
            slot: Some(job.slot.to_u64().unwrap()),
            batch_range_begin_epoch: job.batch_range_begin_epoch.to_u64(),
            batch_range_end_epoch: job.batch_range_end_epoch.to_u64(),
        };

        let failed_at_step = job.failed_at_step.unwrap_or(JobStatus::Created);
        let db_clone = db_manager.clone();
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            match job_to_retry.job_type {
                JobType::SyncCommitteeUpdate => {
                    info!(
                        "Requesting retry of failed job {} failed previously at step {}... (sync committee update job for sync committee {})",
                        job_id,
                        failed_at_step.to_string(),
                        helpers::slot_to_sync_committee_id(job.slot.to_u64().unwrap())
                    );
                }
                JobType::EpochBatchUpdate => {
                    info!(
                        "Requesting retry of failed job {} failed previously at step {} ... (batch epoch update job for epochs from {} to {})",
                        job_id,
                        failed_at_step.to_string(),
                        job.batch_range_begin_epoch,
                        job.batch_range_end_epoch
                    );
                }
            }

            let _ = db_clone
                .update_job_status(job_id, failed_at_step.clone())
                .await;
            if failed_at_step != JobStatus::OffchainComputationFinished
                && failed_at_step != JobStatus::ReadyToBroadcastOnchain
                && failed_at_step != JobStatus::ProofVerifyCalledOnchain
            // These jobs are done sequentially, not in parallel
            {
                if tx_clone.send(job_to_retry).await.is_err() {
                    // return Err("Failed to send job".into());
                    // Update the status to status what was at the error occurene time
                    error!("Error retrying job: {}", job_id);
                }
            }
        });

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

async fn broadcast_onchain_ready_jobs(
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Fetch jobs with the status `ReadyToBroadcastOnchain`
    let jobs = db_manager
        .get_jobs_with_status(JobStatus::ReadyToBroadcastOnchain)
        .await?;

    // Iterate through the jobs and process them
    for job in jobs {
        match job.job_type {
            JobType::EpochBatchUpdate => {
                let circuit_inputs = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                    job.batch_range_begin_epoch.try_into().unwrap(),
                    job.batch_range_end_epoch.try_into().unwrap(),
                )?;

                info!(
                    "[EPOCH BATCH JOB] Calling epoch batch update onchain for epochs range from {} to {}...",
                    job.batch_range_begin_epoch, job.batch_range_end_epoch
                );

                // Submit to Starknet
                let send_result = bankai
                    .starknet_client
                    .submit_update(circuit_inputs.expected_circuit_outputs, &bankai.config)
                    .await;

                let txhash = match send_result {
                    Ok(txhash) => {
                        info!("[EPOCH BATCH JOB] Transaction sent: {}", txhash);
                        txhash
                    }
                    Err(e) => {
                        error!("[EPOCH BATCH JOB] Transaction sending error: {:?}", e);
                        let _ = db_manager
                            .set_failure_info(job.job_uuid, JobStatus::ReadyToBroadcastOnchain)
                            .await?;
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Error)
                            .await?;

                        continue;
                    }
                };

                info!(
                    "[EPOCH BATCH JOB] Successfully called batch epoch update onchain for job_uuid: {}, txhash: {}",
                    job.job_uuid, txhash.to_hex_string()
                );

                db_manager
                    .update_job_status(job.job_uuid, JobStatus::ProofVerifyCalledOnchain)
                    .await?;

                let _ = db_manager.set_job_txhash(job.job_uuid, txhash).await;

                let confirmation_result =
                    bankai.starknet_client.wait_for_confirmation(txhash).await;

                match confirmation_result {
                    Ok(_) => {
                        info!("[EPOCH BATCH JOB] Transaction is confirmed on-chain!");
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Done)
                            .await?;

                        // Iterate over and insert epochs proofs to db
                        for (index, epoch) in
                            circuit_inputs.circuit_inputs.epochs.iter().enumerate()
                        {
                            println!("Epoch {}: {:?}", index, epoch.expected_circuit_outputs);
                        }
                    }
                    Err(e) => {
                        error!("[EPOCH BATCH JOB] Transaction failed or timed out: {:?}", e);
                        let _ = db_manager
                            .set_failure_info(job.job_uuid, JobStatus::ReadyToBroadcastOnchain)
                            .await?;
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Error)
                            .await?;
                    }
                }

                // let epoch_proof = bankai
                //     .starknet_client
                //     .get_epoch_proof(job.slot.try_into().unwrap(), &bankai.config)
                //     .await
                //     .unwrap();

                // db_manager
                //     .insert_verified_epoch(
                //         job.batch_range_end_epoch.try_into().unwrap(),
                //         epoch_proof,
                //     )
                //     .await?;
            }
            //JobType::EpochUpdate => {}
            JobType::SyncCommitteeUpdate => {
                let sync_committee_update_inputs = SyncCommitteeUpdate::from_json::<
                    SyncCommitteeUpdate,
                >(job.slot.to_u64().unwrap())?;

                let sync_commite_id =
                    helpers::slot_to_sync_committee_id(job.slot.to_u64().unwrap());

                info!(
                    "[SYNC COMMITTEE JOB] Calling sync committee ID {} update onchain...",
                    sync_commite_id
                );

                let send_result = bankai
                    .starknet_client
                    .submit_update(
                        sync_committee_update_inputs.expected_circuit_outputs,
                        &bankai.config,
                    )
                    .await;

                let txhash = match send_result {
                    Ok(txhash) => {
                        info!("[SYNC COMMITTEE JOB] Transaction sent: {}", txhash);
                        txhash
                    }
                    Err(e) => {
                        error!("[SYNC COMMITTEE JOB] Transaction sending error: {:?}", e);
                        let _ = db_manager
                            .set_failure_info(job.job_uuid, JobStatus::ReadyToBroadcastOnchain)
                            .await?;
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Error)
                            .await?;

                        continue;
                    }
                };

                info!("[SYNC COMMITTEE JOB] Successfully called sync committee ID {} update onchain, transaction confirmed, txhash: {}", sync_commite_id, txhash);

                db_manager.set_job_txhash(job.job_uuid, txhash).await?;

                let confirmation_result =
                    bankai.starknet_client.wait_for_confirmation(txhash).await;

                match confirmation_result {
                    Ok(_) => {
                        info!("[SYNC COMMITTEE JOB] Transaction is confirmed on-chain!");
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Done)
                            .await?;

                        // Insert data to DB after successful onchain sync committee verification
                        //let sync_committee_hash = update.expected_circuit_outputs.committee_hash;
                        let sync_committee_hash = match bankai
                            .starknet_client
                            .get_committee_hash(job.slot.to_u64().unwrap(), &bankai.config)
                            .await
                        {
                            Ok(sync_committee_hash) => sync_committee_hash,
                            Err(e) => {
                                // Handle the error
                                return Err(e.into());
                            }
                        };

                        let sync_committee_hash_str = sync_committee_hash
                            .iter()
                            .map(|felt| felt.to_hex_string())
                            .collect::<Vec<_>>()
                            .join("");

                        db_manager
                            .insert_verified_sync_committee(
                                job.slot.to_u64().unwrap(),
                                sync_committee_hash_str,
                            )
                            .await?;
                    }
                    Err(e) => {
                        eprintln!(
                            "[SYNC COMMITTEE JOB] Transaction failed or timed out: {:?}",
                            e
                        );
                        let _ = db_manager
                            .set_failure_info(job.job_uuid, JobStatus::ReadyToBroadcastOnchain)
                            .await?;
                        db_manager
                            .update_job_status(job.job_uuid, JobStatus::Error)
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}

// mpsc jobs //
async fn process_job(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut current_status = job.job_status.clone();
    let job_data = db_manager.get_job_by_id(job.job_id).await?.unwrap();
    let mut batch_id = job_data
        .atlantic_proof_generate_batch_id
        .unwrap_or("".to_string());
    let mut wrapping_batch_id = job_data
        .atlantic_proof_wrapper_batch_id
        .unwrap_or("".to_string());
    loop {
        match job.job_type {
            JobType::SyncCommitteeUpdate => {
                // Sync committee job
                let updated_committee_slot = job.slot.unwrap();
                let update_committee_id =
                    helpers::get_sync_committee_id_by_slot(updated_committee_slot);
                match current_status {
                    JobStatus::Created => {
                        info!("[SYNC COMMITTEE JOB] Started processing sync committee job: {} for sync committee ID: {} (Slot: {})",
                        job.job_id, update_committee_id, updated_committee_slot );

                        let sync_committe_update_program_inputs = bankai
                            .get_sync_committee_update(updated_committee_slot.try_into().unwrap())
                            .await?;

                        info!(
                            "[SYNC COMMITTEE JOB] Sync committee update program inputs generated: {:?}",
                            sync_committe_update_program_inputs
                        );

                        let input_path = sync_committe_update_program_inputs.export();
                        info!(
                            "[SYNC COMMITTEE JOB] Circuit inputs saved at {:?}",
                            input_path
                        );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                            .await?;

                        current_status = JobStatus::ProgramInputsPrepared;
                    }
                    JobStatus::ProgramInputsPrepared | JobStatus::StartedTraceGeneration => {
                        let sync_committe_update_program_inputs =
                            SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(
                                job.slot.unwrap(),
                            )?;
                        info!(
                                "[SYNC COMMITTEE JOB] Starting Cairo execution and PIE generation for Sync Committee: {}...",
                                update_committee_id
                            );

                        CairoRunner::generate_pie(
                            &sync_committe_update_program_inputs,
                            &bankai.config,
                            Some(db_manager.clone()),
                            Some(job.job_id),
                        )
                        .await?;

                        db_manager
                            .update_job_status(job.job_id, JobStatus::PieGenerated)
                            .await?;

                        info!(
                        "[SYNC COMMITTEE JOB] Pie generated successfully for Sync Committee: {}...",
                        update_committee_id
                    );

                        current_status = JobStatus::PieGenerated;
                    }
                    JobStatus::PieGenerated => {
                        let sync_committe_update_program_inputs =
                            SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(
                                job.slot.unwrap(),
                            )?;

                        info!("[SYNC COMMITTEE JOB] Sending proof generation query to Atlantic...");

                        batch_id = bankai
                            .atlantic_client
                            .submit_batch(sync_committe_update_program_inputs)
                            .await?;

                        db_manager
                            .update_job_status(job.job_id, JobStatus::AtlanticProofRequested)
                            .await?;
                        db_manager
                            .set_atlantic_job_queryid(
                                job.job_id,
                                batch_id.clone(),
                                AtlanticJobType::ProofGeneration,
                            )
                            .await?;

                        info!( "[SYNC COMMITTEE JOB] Proof generation batch submitted to atlantic. QueryID: {}",
                            batch_id );

                        current_status = JobStatus::AtlanticProofRequested;
                    }
                    JobStatus::AtlanticProofRequested | JobStatus::AtlanticProofRetrieved => {
                        // Pool for Atlantic execution done
                        info!(
                        "[SYNC COMMITTEE JOB] Waiting for completion of Atlantic job. QueryID: {}",
                        batch_id
                    );
                        bankai
                            .atlantic_client
                            .poll_batch_status_until_done(
                                &batch_id,
                                Duration::new(10, 0),
                                usize::MAX,
                            )
                            .await?;

                        info!(
                            "[SYNC COMMITTEE JOB] Proof generation done by Atlantic. QueryID: {}",
                            batch_id
                        );

                        let proof = bankai
                            .atlantic_client
                            .fetch_proof(batch_id.as_str())
                            .await?;

                        info!(
                            "[SYNC COMMITTEE JOB] Proof retrieved from Atlantic. QueryID: {}",
                            batch_id
                        );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::AtlanticProofRetrieved)
                            .await?;

                        // Submit wrapped proof request
                        info!("[SYNC COMMITTEE JOB] Sending proof wrapping query to Atlantic..");
                        wrapping_batch_id =
                            bankai.atlantic_client.submit_wrapped_proof(proof).await?;
                        info!(
                                    "[SYNC COMMITTEE JOB] Proof wrapping query submitted to Atlantic. Wrapping QueryID: {}",
                                    wrapping_batch_id
                                );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::WrapProofRequested)
                            .await?;
                        db_manager
                            .set_atlantic_job_queryid(
                                job.job_id,
                                wrapping_batch_id.clone(),
                                AtlanticJobType::ProofWrapping,
                            )
                            .await?;

                        current_status = JobStatus::WrapProofRequested;
                    }
                    JobStatus::WrapProofRequested => {
                        info!(
                            "[SYNC COMMITTEE JOB] Waiting for completion of Atlantic proof wrappinf job. QueryID: {}",
                            wrapping_batch_id
                        );
                        // Pool for Atlantic execution done
                        bankai
                            .atlantic_client
                            .poll_batch_status_until_done(
                                &wrapping_batch_id,
                                Duration::new(10, 0),
                                usize::MAX,
                            )
                            .await?;

                        db_manager
                            .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                            .await?;

                        info!("[SYNC COMMITTEE JOB] Proof wrapping done by Atlantic. Fact registered on Integrity. Wrapping QueryID: {}", wrapping_batch_id);

                        db_manager
                            .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                            .await?;
                        break;
                    }
                    _ => {
                        error!("[SYNC COMMITTEE JOB] Unexpected behaviour");
                        break;
                    }
                }
            }

            JobType::EpochBatchUpdate => {
                match current_status {
                    JobStatus::Created => {
                        info!("[BATCH EPOCH JOB] Preparing inputs for program for epochs from {} to {}...", job.batch_range_begin_epoch.unwrap(), job.batch_range_end_epoch.unwrap());
                        let circuit_inputs = EpochUpdateBatch::new_by_epoch_range(
                            &bankai,
                            db_manager.clone(),
                            job.batch_range_begin_epoch.unwrap(),
                            job.batch_range_end_epoch.unwrap(),
                        )
                        .await?;

                        let input_path = circuit_inputs.export();
                        info!("[BATCH EPOCH JOB] Circuit inputs saved at {:?}", input_path);

                        db_manager
                            .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                            .await?;

                        current_status = JobStatus::ProgramInputsPrepared;
                    }
                    JobStatus::ProgramInputsPrepared | JobStatus::StartedTraceGeneration => {
                        let circuit_inputs = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                            job.batch_range_begin_epoch.unwrap(),
                            job.batch_range_end_epoch.unwrap(),
                        )?;

                        info!("[BATCH EPOCH JOB] Starting trace generation...");

                        CairoRunner::generate_pie(
                            &circuit_inputs,
                            &bankai.config,
                            Some(db_manager.clone()),
                            Some(job.job_id),
                        )
                        .await?;

                        db_manager
                            .update_job_status(job.job_id, JobStatus::PieGenerated)
                            .await?;

                        current_status = JobStatus::PieGenerated;
                    }
                    JobStatus::PieGenerated => {
                        let circuit_inputs = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                            job.batch_range_begin_epoch.unwrap(),
                            job.batch_range_end_epoch.unwrap(),
                        )?;

                        info!("[BATCH EPOCH JOB] Uploading PIE and sending proof generation request to Atlantic...");

                        batch_id = bankai.atlantic_client.submit_batch(circuit_inputs).await?;

                        info!(
                            "[BATCH EPOCH JOB] Proof generation batch submitted to Atlantic. QueryID: {}",
                            batch_id
                        );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::AtlanticProofRequested)
                            .await?;
                        db_manager
                            .set_atlantic_job_queryid(
                                job.job_id,
                                batch_id.clone(),
                                AtlanticJobType::ProofGeneration,
                            )
                            .await?;

                        current_status = JobStatus::AtlanticProofRequested;
                    }
                    JobStatus::AtlanticProofRequested | JobStatus::AtlanticProofRetrieved => {
                        // Pool for Atlantic execution done
                        info!(
                        "[BATCH EPOCH JOB] Waiting for completion of Atlantic proof generation job. QueryID: {}",
                        batch_id
                        );

                        bankai
                            .atlantic_client
                            .poll_batch_status_until_done(
                                &batch_id,
                                Duration::new(10, 0),
                                usize::MAX,
                            )
                            .await?;

                        info!(
                            "[BATCH EPOCH JOB] Proof generation done by Atlantic. QueryID: {}",
                            batch_id
                        );

                        let proof = bankai
                            .atlantic_client
                            .fetch_proof(batch_id.as_str())
                            .await?;

                        info!(
                            "[BATCH EPOCH JOB] Proof retrieved from Atlantic. QueryID: {}",
                            batch_id
                        );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::AtlanticProofRetrieved)
                            .await?;

                        // 5) Submit wrapped proof request
                        info!(
                            "[BATCH EPOCH JOB] Uploading proof and sending wrapping query to Atlantic.."
                        );
                        wrapping_batch_id =
                            bankai.atlantic_client.submit_wrapped_proof(proof).await?;
                        info!(
                                    "[BATCH EPOCH JOB] Proof wrapping query submitted to Atlantic. Wrapping QueryID: {}",
                                    wrapping_batch_id
                                );

                        db_manager
                            .update_job_status(job.job_id, JobStatus::WrapProofRequested)
                            .await?;

                        db_manager
                            .set_atlantic_job_queryid(
                                job.job_id,
                                wrapping_batch_id.clone(),
                                AtlanticJobType::ProofWrapping,
                            )
                            .await?;

                        current_status = JobStatus::WrapProofRequested;
                    }
                    JobStatus::WrapProofRequested => {
                        // Pool for Atlantic execution done
                        info!(
                        "[BATCH EPOCH JOB] Waiting for completion of Atlantic proof wrapping job. QueryID: {}",
                         wrapping_batch_id
                        );

                        bankai
                            .atlantic_client
                            .poll_batch_status_until_done(
                                &wrapping_batch_id,
                                Duration::new(10, 0),
                                usize::MAX,
                            )
                            .await?;

                        db_manager
                            .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                            .await?;

                        info!("[BATCH EPOCH JOB] Proof wrapping done by Atlantic. Fact registered on Integrity. Wrapping QueryID: {}", wrapping_batch_id);

                        db_manager
                            .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                            .await?;

                        break;
                    }
                    _ => {
                        error!("[BATCH EPOCH JOB] Unexpected behaviour");
                        break;
                    }
                }
            }
        }
    }
    Ok(())
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

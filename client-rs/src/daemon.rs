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
use alloy_primitives::FixedBytes;
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
use num_traits::cast::ToPrimitive;
use reqwest;
use starknet::core::types::Felt;
use state::check_env_vars;
use state::{AppState, Job};
use state::{AtlanticJobType, Error, JobStatus, JobType};
use std::env;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::StreamExt;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use traits::Provable;
use utils::{
    cairo_runner::CairoRunner,
    database_manager::{DatabaseManager, JobSchema},
};
//use std::error::Error as StdError;
use epoch_batch::EpochUpdateBatch;
use routes::{
    handle_get_epoch_update, handle_get_latest_verified_slot, handle_get_merkle_paths_for_epoch,
    handle_get_status,
};
use std::net::SocketAddr;
use sync_committee::SyncCommitteeUpdate;
use tokio::time::Duration;
use uuid::Uuid;

// Since beacon chain RPCs have different response structure (quicknode responds different than nidereal) we use this event extraction logic
fn extract_json(event_text: &str) -> Option<String> {
    for line in event_text.lines() {
        if line.starts_with("data:") {
            // Extract the JSON after "data:"
            return Some(line.trim_start_matches("data:").trim().to_string());
        }
    }
    None
}

#[tokio::main]
//async fn main() {
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env.sepolia file
    from_filename(".env.sepolia").ok();

    let slot_listener_toggle = true;

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

    //let database_host = env::var("DATABASE_HOST").expect("DATABASE_HOST must be set");
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
    let bankai_for_listener = bankai.clone();

    let tx_for_listener = tx.clone();

    let app_state: AppState = AppState {
        db_manager: db_manager.clone(),
        tx,
        bankai: bankai.clone(),
    };

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
                        let _ = db_clone.update_job_status(job_id, JobStatus::Error).await;
                        error!("Error processing job {}: {}", job_id, e);
                    }
                }
            });
        }
    });

    let app = Router::new()
        .route("/status", get(handle_get_status))
        //.route("/get-epoch-proof/:slot", get(handle_get_epoch_proof))
        //.route("/get-committee-hash/:committee_id", get(handle_get_committee_hash))
        .route(
            "/get_merkle_paths_for_epoch/:epoch_id",
            get(handle_get_merkle_paths_for_epoch),
        )
        .route(
            "/debug/get-epoch-update/:slot",
            get(handle_get_epoch_update),
        )
        .route(
            "/debug/get-latest-verified-slot",
            get(handle_get_latest_verified_slot),
        )
        // .route("/debug/get-job-status", get(handle_get_job_status))
        // .route("/get-merkle-inclusion-proof", get(handle_get_merkle_inclusion_proof))
        .layer(DefaultBodyLimit::disable())
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http()), // Example: for logging/tracing
        )
        .with_state(app_state);

    let addr = "0.0.0.0:3000".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Bankai RPC HTTP server is listening on http://{}", addr);

    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    // Listen for the new slots on BeaconChain
    // Create an HTTP client
    let http_stream_client = reqwest::Client::new();

    // Send the request to the Beacon node
    let response = http_stream_client
        .get(&events_endpoint)
        .send()
        .await
        .unwrap();

    //let db_client = Arc::new(&db_client);
    if slot_listener_toggle {
        task::spawn({
            async move {
                // Check if response is successful; if not, bail out early
                // TODO: need to implement resilience and potentialy use multiple providers (implement something like fallbackprovider functionality in ethers), handle reconnection if connection is lost for various reasons
                if !response.status().is_success() {
                    error!("Failed to connect: HTTP {}", response.status());
                    return;
                }

                info!("Listening for new slots, epochs and sync committee updates...");
                let mut stream = response.bytes_stream();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            if let Ok(event_text) = String::from_utf8(bytes.to_vec()) {
                                // Preprocess the event text
                                if let Some(json_data) = extract_json(&event_text) {
                                    match serde_json::from_str::<HeadEvent>(&json_data) {
                                        Ok(parsed_event) => {
                                            let epoch_id =
                                                helpers::slot_to_epoch_id(parsed_event.slot);
                                            let sync_committee_id =
                                                helpers::slot_to_sync_committee_id(
                                                    parsed_event.slot,
                                                );
                                            info!(
                                                "New slot event detected: {} |  Block: {} | Epoch: {} | Sync committee: {} | Is epoch transition: {}",
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
                        Err(err) => {
                            warn!("Error reading event stream: {}", err);
                        }
                    }
                }
            }
        });
    }

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
    let epoch_id = helpers::slot_to_epoch_id(parsed_event.slot);
    let sync_committee_id = helpers::slot_to_sync_committee_id(parsed_event.slot);

    let latest_epoch_slot = bankai
        .starknet_client
        .get_latest_epoch_slot(&bankai.config)
        .await
        .unwrap()
        .to_u64()
        .unwrap();

    let latest_verified_epoch_id = helpers::slot_to_epoch_id(latest_epoch_slot);
    let epochs_behind = epoch_id - latest_verified_epoch_id;

    // We getting the last slot in progress to determine next slots to prove
    let mut last_slot_in_progress: u64 = 0;
    // /let mut last_epoch_in_progress: u64 = 0;
    let mut last_sync_committee_in_progress: u64 = 0;
    // match db_manager.get_latest_slot_id_in_progress().await {
    //     Ok(Some(slot)) => {
    //         last_slot_in_progress = slot.to_u64().unwrap();
    //         last_epoch_in_progress = helpers::slot_to_epoch_id(last_slot_in_progress);
    //         last_sync_committee_in_progress =
    //             helpers::slot_to_sync_committee_id(last_slot_in_progress);
    //         info!(
    //             "Latest in progress slot: {}  Epoch: {} Sync committee: {}",
    //             last_slot_in_progress, last_epoch_in_progress, last_sync_committee_in_progress
    //         );
    //     }
    //     Ok(None) => {
    //         warn!("No any in progress slot");
    //     }
    //     Err(e) => {
    //         error!("Error while getting latest in progress slot ID: {}", e);
    //     }
    // }

    let last_epoch_in_progress = db_manager
        .get_latest_epoch_in_progress()
        .await
        .unwrap()
        .unwrap();

    let latest_verified_sync_committee_id = 1;

    if epochs_behind > constants::TARGET_BATCH_SIZE {
        // is_node_in_sync = true;

        warn!(
            "Bankai is out of sync now. Node is {} epochs behind network. Current Beacon Chain state: [Epoch: {} Sync Committee: {}] | Latest verified: [Epoch: {} Sync Committee: {}] | Sync in progress...",
            epochs_behind, epoch_id, sync_committee_id, latest_verified_epoch_id, latest_verified_sync_committee_id
        );

        // Check if we have in progress all epochs that need to be processed, if no, run job
        if last_epoch_in_progress < (epoch_id - constants::TARGET_BATCH_SIZE) {
            // And chceck how many jobs are already in progress and if we fit in the limit
            let in_progress_jobs_count = db_manager.count_jobs_in_progress().await.unwrap();
            if in_progress_jobs_count.unwrap().to_u64().unwrap()
                >= constants::MAX_CONCURRENT_JOBS_IN_PROGRESS
            {
                info!(
                    "Currently not starting new job, MAX_CONCURRENT_JOBS_IN_PROGRESS limit reached"
                );
                return;
            }

            match run_batch_epoch_update_job(
                db_manager.clone(),
                last_slot_in_progress + (constants::SLOTS_PER_EPOCH * constants::TARGET_BATCH_SIZE),
                last_epoch_in_progress,
                last_epoch_in_progress + constants::TARGET_BATCH_SIZE,
                tx.clone(),
            )
            .await
            {
                // Insert new job record to DB
                Ok(()) => {}
                Err(e) => {
                    error!("Error while creating job: {}", e);
                }
            };
        } else {
            debug!("All reqired jobs are now queued and processing");
        }

        // let epoch_update = EpochUpdateBatch::new_by_slot(
        //     &bankai,
        //     &db_client_for_listener.clone(),
        //     last_slot_in_progress
        //         + constants::SLOTS_PER_EPOCH,
        // )
        // .await?;
    } else if epochs_behind == constants::TARGET_BATCH_SIZE {
        // This is when we are synced properly and new epoch batch needs to be inserted
        info!(
            "Starting provessing next epoch batch. Current Beacon Chain epoch: {} Latest verified epoch: {}",
            epoch_id, latest_verified_epoch_id
        );
    }

    // Check if sync committee update is needed
    //sync_committee_id

    if latest_epoch_slot % constants::SLOTS_PER_SYNC_COMMITTEE == 0 {}

    //return;

    // When we doing EpochBatchUpdate the slot is latest_batch_output
    // So for each batch update we takin into account effectiviely the latest slot from given batch

    //let db_client = db_client.clone();

    //evaluate_jobs_statuses(db_manager.clone()).await;
    broadcast_onchain_ready_jobs(db_manager.clone(), bankai.clone()).await;

    // We can do all circuit computations up to latest slot in advance, but the onchain broadcasts must be send in correct order
    // By correct order mean that within the same sync committe the epochs are not needed to be broadcasted in order
    // but the order of sync_commite_update->epoch_update must be correct, we firstly need to have correct sync committe veryfied
    // before we verify epoch "belonging" to this sync committee

    if parsed_event.epoch_transition {
        //info!("Beacon Chain epoch transition detected. New epoch: {} | Starting processing epoch proving...", epoch_id);
        info!(
            "Beacon Chain epoch transition detected. New epoch: {}",
            epoch_id
        );

        // Check also now if slot is the moment of switch to new sync committee set
        if parsed_event.slot % constants::SLOTS_PER_SYNC_COMMITTEE == 0 {
            info!(
                "Beacon Chain sync committee rotation occured. Slot {} | Sync committee id: {}",
                parsed_event.slot, sync_committee_id
            );
        }

        //     let job_id = Uuid::new_v4();
        //     let job = Job {
        //         job_id: job_id.clone(),evaluate_jobs_statuses
        //         job_type: JobType::EpochBatchUpdate,
        //         job_status: JobStatus::Created,
        //         slot: parsed_event.slot, // It is the last slot for given batch
        //     };

        //     let db_client = db_client_for_listener.clone();
        //     match create_job(db_client, job.clone()).await {
        //         // Insert new job record to DB
        //         Ok(()) => {
        //             // Handle success
        //             info!("Job created successfully with ID: {}", job_id);
        //             if tx_for_task.send(job).await.is_err() {
        //                 error!("Failed to send job.");
        //             }
        //             // If starting committee update job, first ensule that the corresponding slot is registered in contract
        //         }
        //         Err(e) => {
        //             // Handle the error
        //             error!("Error creating job: {}", e);
        //         }
        //     }
    }
}

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
        slot,
        batch_range_begin_epoch: Some(batch_range_begin_epoch),
        batch_range_end_epoch: Some(batch_range_end_epoch),
    };

    match db_manager.create_job(job.clone()).await {
        // Insert new job record to DB
        Ok(()) => {
            // Handle success
            info!(
                "[EPOCH BATCH UPDATE] Job created successfully with ID: {}",
                job_id
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
    slot: u64,
    tx: mpsc::Sender<Job>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let job_id = Uuid::new_v4();
    let job = Job {
        job_id: job_id.clone(),
        job_type: JobType::SyncCommitteeUpdate,
        job_status: JobStatus::Created,
        slot,
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

// async fn evaluate_jobs_statuses(
//     db_manager: Arc<DatabaseManager>,
//     last_verified_epoch: u64,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//     // The purpose of this function is to manage the sequential nature of onchain verification of epochs and sync committees
//     // Firstly we get all jobs with status OFFCHAIN_COMPUTATION_FINISHED
//     let jobs = db_manager
//         .get_compute_finsihed_jobs_to_proccess_onchain_call(last_verified_epoch)
//         .await?;

//     // Iterate through the jobs and process them
//     for job in jobs {
//         match job.job_type {
//             JobType::EpochBatchUpdate => {
//                 let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
//                     job.batch_range_begin_epoch.try_into().unwrap(),
//                     job.batch_range_end_epoch.try_into().unwrap(),
//                 )?;

//                 println!(
//                     "Successfully submitted batch epoch update for job_uuid: {}",
//                     job.job_uuid
//                 );
//             }
//             JobType::EpochUpdate => {}
//             JobType::SyncCommitteeUpdate => {}
//         }
//     }

//     Ok(())
// }

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
                let update = EpochUpdateBatch::from_json::<EpochUpdateBatch>(
                    job.batch_range_begin_epoch.try_into().unwrap(),
                    job.batch_range_end_epoch.try_into().unwrap(),
                )?;

                info!(
                    "[SYNC COMMITTEE JOB] Calling epoch batch update onchain for epochs range from {} to {}...",
                    job.batch_range_begin_epoch, job.batch_range_end_epoch
                );

                // Submit to Starknet
                let txhash = bankai
                    .starknet_client
                    .submit_update(update.expected_circuit_outputs, &bankai.config)
                    .await?;

                println!(
                    "[EPOCH BATCH JOB] Successfully called batch epoch update onchain for job_uuid: {}, txhash: {}",
                    job.job_uuid, txhash
                );

                let epoch_proof = bankai
                    .starknet_client
                    .get_epoch_proof(job.slot.try_into().unwrap(), &bankai.config);

                // db_manager
                //     .insert_verified_epochs_batch(job.slot / 0x2000, epoch_proof)
                //     .await?;
            }
            JobType::EpochUpdate => {}
            JobType::SyncCommitteeUpdate => {
                let update = SyncCommitteeUpdate::from_json::<SyncCommitteeUpdate>(
                    job.slot.to_u64().unwrap(),
                )?;

                let sync_commite_id =
                    helpers::slot_to_sync_committee_id(job.slot.to_u64().unwrap());

                info!(
                    "[SYNC COMMITTEE JOB] Calling sync committee ID {} update onchain...",
                    sync_commite_id
                );

                let txhash = bankai
                    .starknet_client
                    .submit_update(update.expected_circuit_outputs, &bankai.config)
                    .await?;

                info!("[SYNC COMMITTEE JOB] Successfully called sync committee ID {} update onchain, transaction confirmed, txhash: {}", sync_commite_id, txhash);

                db_manager.set_job_txhash(job.job_uuid, txhash).await?;

                // Insert data to DB after successful onchain sync committee verification
                //let sync_committee_hash = update.expected_circuit_outputs.committee_hash;
                let sync_committee_hash = match bankai
                    .starknet_client
                    .get_committee_hash(job.slot.to_u64().unwrap(), &bankai.config)
                    .await
                {
                    Ok((sync_committee_hash)) => sync_committee_hash,
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
        }
    }

    Ok(())
}

// async fn worker_task(mut rx: Receiver<Uuid>, db_client: Client) -> Result<(), Box<dyn Error>> {
//     while let Some(job_id) = rx.recv().await {
//         println!("Worker received job {job_id}");

//         // 4a) Check current status in DB
//         if let Some(status) = fetch_job_status(&db_client, job_id).await? {
//             match status {
//                 JobStatus::Created => {
//                     println!("Fetching proof for job {job_id}...");
//                     // Then update status
//                     update_job_status(&db_client, job_id, JobStatus::FetchedProof).await?;
//                     println!("Job {job_id} updated to FetchedProof");
//                 }
//                 JobStatus::FetchedProof => {
//                     // Already fetched, maybe do next step...
//                     println!("Job {job_id} is already FetchedProof; ignoring for now.");
//                 }
//                 _ => {
//                     println!("Job {job_id} in status {:?}, no action needed.", status);
//                 }
//             }
//         } else {
//             eprintln!("No job found in DB for ID = {job_id}");
//         }
//     }
//     Ok(())
// }

// mpsc jobs //
async fn process_job(
    job: Job,
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match job.job_type {
        JobType::EpochUpdate => {
            // Epoch job
            info!(
                "[EPOCH JOB] Started processing epoch job: {} for epoch {}",
                job.job_id, job.slot
            );

            //update_job_status(&db_client, job.job_id, JobStatus::Created).await?;

            // 1) Fetch the latest on-chain verified epoch
            // let latest_epoch_slot = bankai
            //     .starknet_client
            //     .get_latest_epoch_slot(&bankai.config)
            //     .await?;

            // info!(
            //     "[EPOCH JOB] Latest onchain verified epoch slot: {}",
            //     latest_epoch_slot
            // );

            //let latest_epoch_slot = ;

            // make sure next_epoch % 32 == 0
            let next_epoch = (u64::try_from(job.slot).unwrap() / constants::SLOTS_PER_EPOCH)
                * constants::SLOTS_PER_EPOCH
                + constants::SLOTS_PER_EPOCH;
            info!(
                "[EPOCH JOB] Fetching Inputs for next Epoch: {}...",
                next_epoch
            );

            // 2) Fetch the proof
            let proof = bankai.get_epoch_proof(next_epoch).await?;
            info!(
                "[EPOCH JOB] Fetched Inputs successfully for Epoch: {}",
                next_epoch
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                .await?;

            // 3) Generate PIE
            info!(
                "[EPOCH JOB] Starting Cairo execution and PIE generation for Epoch: {}...",
                next_epoch
            );

            CairoRunner::generate_pie(&proof, &bankai.config).await?;

            info!(
                "[EPOCH JOB] Pie generated successfully for Epoch: {}...",
                next_epoch
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::PieGenerated)
                .await?;

            // // 4) Submit offchain proof-generation job to Atlantic
            // info!("[EPOCH JOB] Sending proof generation query to Atlantic...");

            // let batch_id = bankai.atlantic_client.submit_batch(proof).await?;

            // info!(
            //     "[EPOCH JOB] Proof generation batch submitted to Atlantic. QueryID: {}",
            //     batch_id
            // );

            // update_job_status(&db_client, job.job_id, JobStatus::OffchainProofRequested).await?;
            // set_atlantic_job_queryid(
            //     &db_client,
            //     job.job_id,
            //     batch_id.clone(),
            //     AtlanticJobType::ProofGeneration,
            // )
            // .await?;

            // // Pool for Atlantic execution done
            // bankai
            //     .atlantic_client
            //     .poll_batch_status_until_done(&batch_id, Duration::new(10, 0), usize::MAX)
            //     .await?;

            // info!(
            //     "[EPOCH JOB] Proof generation done by Atlantic. QueryID: {}",
            //     batch_id
            // );

            // let proof = bankai
            //     .atlantic_client
            //     .fetch_proof(batch_id.as_str())
            //     .await?;

            // info!(
            //     "[EPOCH JOB] Proof retrieved from Atlantic. QueryID: {}",
            //     batch_id
            // );

            // update_job_status(&db_client, job.job_id, JobStatus::OffchainProofRetrieved).await?;

            // // 5) Submit wrapped proof request
            // info!("[EPOCH JOB] Sending proof wrapping query to Atlantic..");
            // let wrapping_batch_id = bankai.atlantic_client.submit_wrapped_proof(proof).await?;
            // info!(
            //     "[EPOCH JOB] Proof wrapping query submitted to Atlantic. Wrapping QueryID: {}",
            //     wrapping_batch_id
            // );

            // update_job_status(&db_client, job.job_id, JobStatus::WrapProofRequested).await?;
            // set_atlantic_job_queryid(
            //     &db_client,
            //     job.job_id,
            //     wrapping_batch_id.clone(),
            //     AtlanticJobType::ProofWrapping,
            // )
            // .await?;

            // // Pool for Atlantic execution done
            // bankai
            //     .atlantic_client
            //     .poll_batch_status_until_done(&wrapping_batch_id, Duration::new(10, 0), usize::MAX)
            //     .await?;

            // update_job_status(&db_client, job.job_id, JobStatus::WrappedProofDone).await?;

            // info!("[EPOCH JOB] Proof wrapping done by Atlantic. Fact registered on Integrity. Wrapping QueryID: {}", wrapping_batch_id);

            // update_job_status(&db_client, job.job_id, JobStatus::VerifiedFactRegistered).await?;

            // // 6) Submit epoch update onchain
            // info!("[EPOCH JOB] Calling epoch update onchain...");
            // let update = EpochUpdate::from_json::<EpochUpdate>(next_epoch)?;

            // let txhash = bankai
            //     .starknet_client
            //     .submit_update(update.expected_circuit_outputs, &bankai.config)
            //     .await?;

            // set_job_txhash(&db_client, job.job_id, txhash).await?;

            // info!("[EPOCH JOB] Successfully submitted epoch update...");

            // update_job_status(&db_client, job.job_id, JobStatus::ProofDecommitmentCalled).await?;

            // Now we can get proof from contract?
            // bankai.starknet_client.get_epoch_proof(
            //     &self,
            //     slot: u64,
            //     config: &BankaiConfig)

            // Insert data to DB after successful onchain epoch verification
            // insert_verified_epoch(&db_client, job.slot / 0x2000, epoch_proof).await?;
        }
        JobType::SyncCommitteeUpdate => {
            // Sync committee job
            info!(
                "[SYNC COMMITTEE JOB] Started processing sync committee job: {} for slot {}",
                job.job_id, job.slot
            );

            let latest_committee_id = bankai
                .starknet_client
                .get_latest_committee_id(&bankai.config)
                .await?;

            info!(
                "[SYNC COMMITTEE JOB] Latest onchain verified sync committee id: {}",
                latest_committee_id
            );

            let latest_epoch = bankai
                .starknet_client
                .get_latest_epoch_slot(&bankai.config)
                .await?;

            let lowest_committee_update_slot = (latest_committee_id) * Felt::from(0x2000);

            if latest_epoch < lowest_committee_update_slot {
                error!("[SYNC COMMITTEE JOB] Sync committee update requires newer epoch verified. The lowest needed slot is {}", lowest_committee_update_slot);
                //return Err(Error::RequiresNewerEpoch(latest_epoch));
            }

            let update = bankai
                .get_sync_committee_update(latest_epoch.try_into().unwrap())
                .await?;

            info!(
                "[SYNC COMMITTEE JOB] Received sync committee update: {:?}",
                update
            );

            info!(
                "[SYNC COMMITTEE JOB] Starting Cairo execution and PIE generation for Sync Committee: {:?}...",
                latest_committee_id
            );

            CairoRunner::generate_pie(&update, &bankai.config).await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::PieGenerated)
                .await?;

            info!(
                "[SYNC COMMITTEE JOB] Pie generated successfully for Sync Committee: {}...",
                latest_committee_id
            );
            info!("[SYNC COMMITTEE JOB] Sending proof generation query to Atlantic...");

            let batch_id = bankai.atlantic_client.submit_batch(update).await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainProofRequested)
                .await?;
            db_manager
                .set_atlantic_job_queryid(
                    job.job_id,
                    batch_id.clone(),
                    AtlanticJobType::ProofGeneration,
                )
                .await?;

            info!(
                "[SYNC COMMITTEE JOB] Proof generation batch submitted to atlantic. QueryID: {}",
                batch_id
            );

            // Pool for Atlantic execution done
            bankai
                .atlantic_client
                .poll_batch_status_until_done(&batch_id, Duration::new(10, 0), usize::MAX)
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
                .update_job_status(job.job_id, JobStatus::OffchainProofRetrieved)
                .await?;

            // 5) Submit wrapped proof request
            info!("[SYNC COMMITTEE JOB] Sending proof wrapping query to Atlantic..");
            let wrapping_batch_id = bankai.atlantic_client.submit_wrapped_proof(proof).await?;
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

            // Pool for Atlantic execution done
            bankai
                .atlantic_client
                .poll_batch_status_until_done(&wrapping_batch_id, Duration::new(10, 0), usize::MAX)
                .await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            info!("[SYNC COMMITTEE JOB] Proof wrapping done by Atlantic. Fact registered on Integrity. Wrapping QueryID: {}", wrapping_batch_id);

            db_manager
                .update_job_status(job.job_id, JobStatus::VerifiedFactRegistered)
                .await?;
        }
        JobType::EpochBatchUpdate => {
            info!("[BATCH EPOCH JOB] Preparing inputs for program...");

            let proof = EpochUpdateBatch::new_by_epoch_range(
                &bankai,
                db_manager.clone(),
                job.batch_range_begin_epoch.unwrap(),
                job.batch_range_end_epoch.unwrap(),
            )
            .await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::ProgramInputsPrepared)
                .await?;

            info!("[BATCH EPOCH JOB] Starting trace generation...");

            CairoRunner::generate_pie(&proof, &bankai.config).await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::PieGenerated)
                .await?;

            info!("[BATCH EPOCH JOB] Uploading PIE and sending proof generation request to Atlantic...");

            let batch_id = bankai.atlantic_client.submit_batch(proof).await?;

            info!(
                "[BATCH EPOCH JOB] Proof generation batch submitted to Atlantic. QueryID: {}",
                batch_id
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainProofRequested)
                .await?;
            db_manager
                .set_atlantic_job_queryid(
                    job.job_id,
                    batch_id.clone(),
                    AtlanticJobType::ProofGeneration,
                )
                .await?;

            // Pool for Atlantic execution done
            bankai
                .atlantic_client
                .poll_batch_status_until_done(&batch_id, Duration::new(10, 0), usize::MAX)
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
                "[EPOCH JOB] Proof retrieved from Atlantic. QueryID: {}",
                batch_id
            );

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainProofRetrieved)
                .await?;

            // 5) Submit wrapped proof request
            info!("[EPOCH JOB] Uploading proof and sending wrapping query to Atlantic..");
            let wrapping_batch_id = bankai.atlantic_client.submit_wrapped_proof(proof).await?;
            info!(
                "[EPOCH JOB] Proof wrapping query submitted to Atlantic. Wrapping QueryID: {}",
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

            // Pool for Atlantic execution done
            bankai
                .atlantic_client
                .poll_batch_status_until_done(&wrapping_batch_id, Duration::new(10, 0), usize::MAX)
                .await?;

            db_manager
                .update_job_status(job.job_id, JobStatus::WrappedProofDone)
                .await?;

            info!("[EPOCH JOB] Proof wrapping done by Atlantic. Fact registered on Integrity. Wrapping QueryID: {}", wrapping_batch_id);

            db_manager
                .update_job_status(job.job_id, JobStatus::OffchainComputationFinished)
                .await?;
        }
    }

    Ok(())
}

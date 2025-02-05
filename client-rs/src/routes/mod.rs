use crate::state::AppState;
use alloy_primitives::map::HashMap;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use num_traits::cast::ToPrimitive;
use serde_json::{json, Value};
use tracing::error;
use uuid::Uuid;

pub mod dashboard;

//  RPC requests handling functions //
pub async fn handle_root_route(State(_state): State<AppState>) -> impl IntoResponse {
    Json(json!({ "success": true, "message": "Bankai daemon running" }))
}

// Handler for GET /status
pub async fn handle_get_status(State(state): State<AppState>) -> impl IntoResponse {
    let last_epoch_in_progress = match state.db_manager.get_latest_epoch_in_progress().await {
        Ok(Some(epoch)) => {
            let last_epoch_in_progress = epoch.to_u64().unwrap();
            last_epoch_in_progress
        }
        Ok(None) => 0,
        Err(_) => 0,
    };
    let in_progress_jobs_count = state.db_manager.count_jobs_in_progress().await.unwrap();
    let last_sync_committee_in_progress = state
        .db_manager
        .get_latest_sync_committee_in_progress()
        .await
        .unwrap()
        .unwrap();

    // let beacon_chain_state = state
    //     .db_manager
    //     .get_latest_known_beacon_chain_state()
    //     .await
    //     .unwrap();
    //
    let jobs_status_counts = state
        .db_manager
        .get_jobs_count_by_status()
        .await
        .unwrap_or_default();

    let mut jobs_status_map = HashMap::new();
    for job_status_count in jobs_status_counts {
        jobs_status_map.insert(job_status_count.status.to_string(), job_status_count.count);
    }

    Json(json!({ "success": true, "details": {
        "last_epoch_in_progress": last_epoch_in_progress,
        "last_sync_committee_in_progress": last_sync_committee_in_progress,
        "jobs_in_progress_count": in_progress_jobs_count,
        "jobs_statuses": jobs_status_map
    } }))
}

// // Handler for GET /epoch/:slot
// pub async fn handle_get_epoch_update(
//     Path(slot): Path<u64>,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     match state.bankai.get_epoch_proof(slot).await {
//         Ok(epoch_update) => {
//             // Convert the data to `serde_json::Value`
//             let value: Value = serde_json::to_value(epoch_update).unwrap_or_else(|err| {
//                 eprintln!("Failed to serialize EpochUpdate: {:?}", err);
//                 json!({ "error": "Internal server error" })
//             });
//             Json(value)
//         }
//         Err(err) => {
//             eprintln!("Failed to fetch proof: {:?}", err);
//             Json(json!({ "error": "Failed to fetch proof" }))
//         }
//     }
// }

pub async fn handle_get_epoch_proof(
    Path(slot): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .bankai
        .starknet_client
        .get_epoch_proof(slot, &state.bankai.config)
        .await
    {
        Ok(epoch_update) => {
            // Convert `EpochUpdate` to `serde_json::Value`
            let value = serde_json::to_value(epoch_update).unwrap_or_else(|err| {
                eprintln!("Failed to serialize EpochUpdate: {:?}", err);
                json!({ "error": "Internal server error" })
            });
            Json(value)
        }
        Err(err) => {
            eprintln!("Failed to fetch proof: {:?}", err);
            Json(json!({ "error": "Failed to fetch proof" }))
        }
    }
}

pub async fn handle_get_committee_hash(
    Path(committee_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .bankai
        .starknet_client
        .get_committee_hash(committee_id, &state.bankai.config)
        .await
    {
        Ok(committee_hash) => {
            // Convert `EpochUpdate` to `serde_json::Value`
            let value = serde_json::to_value(committee_hash).unwrap_or_else(|err| {
                eprintln!("Failed to serialize EpochUpdate: {:?}", err);
                json!({ "error": "Internal server error" })
            });
            Json(value)
        }
        Err(err) => {
            eprintln!("Failed to fetch proof: {:?}", err);
            Json(json!({ "error": "Failed to fetch proof" }))
        }
    }
}

pub async fn handle_get_latest_verified_slot(State(state): State<AppState>) -> impl IntoResponse {
    match state
        .bankai
        .starknet_client
        .get_latest_epoch_slot(&state.bankai.config)
        .await
    {
        Ok(latest_epoch) => {
            // Convert `Felt` to a string and parse it as a hexadecimal number
            let hex_string = latest_epoch.to_string(); // Ensure this converts to a "0x..." string
            match u64::from_str_radix(hex_string.trim_start_matches("0x"), 16) {
                Ok(decimal_epoch) => Json(json!({ "latest_verified_slot": decimal_epoch })),
                Err(err) => {
                    eprintln!("Failed to parse latest_epoch as decimal: {:?}", err);
                    Json(json!({ "error": "Invalid epoch format" }))
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch latest epoch: {:?}", err);
            Json(json!({ "error": "Failed to fetch latest epoch" }))
        }
    }
}

pub async fn handle_get_latest_verified_committee(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .bankai
        .starknet_client
        .get_latest_committee_id(&state.bankai.config)
        .await
    {
        Ok(latest_epoch) => {
            // Convert `Felt` to a string and parse it as a hexadecimal number
            let hex_string = latest_epoch.to_string(); // Ensure this converts to a "0x..." string
            match u64::from_str_radix(hex_string.trim_start_matches("0x"), 16) {
                Ok(decimal_epoch) => Json(json!({ "latest_verified_epoch": decimal_epoch })),
                Err(err) => {
                    eprintln!("Failed to parse latest_epoch as decimal: {:?}", err);
                    Json(json!({ "error": "Invalid epoch format" }))
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch latest epoch: {:?}", err);
            Json(json!({ "error": "Failed to fetch latest epoch" }))
        }
    }
}

pub async fn handle_get_job_status(
    Path(job_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .db_manager
        .fetch_job_status(Uuid::parse_str(job_id.to_string().as_str()).unwrap())
        .await
    {
        Ok(Some(job_status)) => Json(json!({ "status": job_status.to_string()})),
        Ok(None) => Json(json!({ "error": "Job not found" })),
        Err(err) => {
            eprintln!("Failed to fetch job status: {:?}", err);
            Json(json!({ "error": "Failed to fetch job status" }))
        }
    }
}

pub async fn handle_get_merkle_paths_for_epoch(
    Path(epoch_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.db_manager.get_merkle_paths_for_epoch(epoch_id).await {
        Ok(merkle_paths) => {
            if merkle_paths.len() > 0 {
                Json(json!({ "epoch_id": epoch_id, "merkle_paths": merkle_paths }))
            } else {
                Json(json!({ "error": "Epoch not available now" }))
            }
        }
        Err(err) => {
            error!("Failed to fetch merkle paths epoch: {:?}", err);
            Json(json!({ "error": "Failed to fetch latest epoch" }))
        }
    }
}

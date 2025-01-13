use crate::state::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde_json::{json, Value};
use tracing::{error, info, trace, warn, Level};

//  RPC requests handling functions //

// Handler for GET /status
pub async fn handle_get_status(State(_state): State<AppState>) -> impl IntoResponse {
    Json(json!({ "success": true }))
}

// Handler for GET /epoch/:slot
pub async fn handle_get_epoch_update(
    Path(slot): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.bankai.get_epoch_proof(slot).await {
        Ok(epoch_update) => {
            // Convert the data to `serde_json::Value`
            let value: Value = serde_json::to_value(epoch_update).unwrap_or_else(|err| {
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

// async fn handle_get_epoch_proof(
//     Path(slot): Path<u64>,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     match state.bankai.starknet_client.get_epoch_proof(slot).await {
//         Ok(epoch_update) => {
//             // Convert `EpochUpdate` to `serde_json::Value`
//             let value = serde_json::to_value(epoch_update).unwrap_or_else(|err| {
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

// async fn handle_get_committee_hash(
//     Path(committee_id): Path<u64>,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     match state.bankai.starknet_client.get_committee_hash(committee_id).await {
//         Ok(committee_hash) => {
//             // Convert `EpochUpdate` to `serde_json::Value`
//             let value = serde_json::to_value(committee_hash).unwrap_or_else(|err| {
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

// async fn handle_get_job_status(
//     Path(job_id): Path<u64>,
//     State(state): State<AppState>,
// ) -> impl IntoResponse {
//     match fetch_job_status(&state.db_client, job_id).await {
//         Ok(job_status) => Json(job_status),
//         Err(err) => {
//             eprintln!("Failed to fetch job status: {:?}", err);
//             Json(json!({ "error": "Failed to fetch job status" }))
//         }
//     }
// }

pub async fn handle_get_merkle_paths_for_epoch(
    Path(epoch_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match get_merkle_paths_for_epoch(&state.db_client, epoch_id).await {
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

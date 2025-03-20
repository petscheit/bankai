use crate::types::AppState;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use bankai_core::utils::helpers;
use num_traits::ToPrimitive;
use serde_json::json;
use tracing::error;

pub async fn handle_get_decommitment_data_by_epoch(
    Path(epoch_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state.db_manager.get_merkle_paths_for_epoch(epoch_id).await {
        Ok(merkle_paths) => {
            if !merkle_paths.is_empty() {
                let epoch_decommitment_data: bankai_core::types::proofs::epoch_update::EpochDecommitmentData = state
                    .db_manager
                    .get_epoch_decommitment_data(epoch_id)
                    .await
                    .unwrap(); //ExpectedEpochUpdateOutputs

                let circuit_outputs_decommitment_data =
                    epoch_decommitment_data.epoch_update_outputs;

                Json(json!({
                    "epoch_id": epoch_id,
                    "decommitment_data_for_epoch": {
                        "merkle_tree": {
                            "epoch_index": epoch_decommitment_data.epoch_index,
                            "batch_root": epoch_decommitment_data.batch_root,
                            "path": merkle_paths,
                        },
                        "circuit_outputs": circuit_outputs_decommitment_data
                    }
                }))
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

pub async fn handle_get_decommitment_data_by_slot(
    Path(slot_id): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let epoch_id = helpers::slot_to_epoch_id(slot_id.to_u64().unwrap());

    handle_get_decommitment_data_by_epoch(Path(epoch_id.to_i32().unwrap()), State(state)).await
}

pub async fn handle_get_decommitment_data_by_execution_height(
    Path(execution_height): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let epoch_id = state
        .db_manager
        .get_verified_epoch_by_execution_height(execution_height)
        .await
        .unwrap()
        .unwrap();

    handle_get_decommitment_data_by_epoch(Path(epoch_id.to_i32().unwrap()), State(state)).await
}

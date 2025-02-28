use crate::handlers::epoch_decommitment::{
    handle_get_decommitment_data_by_epoch, handle_get_decommitment_data_by_slot,
    handle_get_decommitment_data_by_execution_height,
};
use crate::AppState;
use axum::{routing::get, Router};
use thiserror::Error;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/get_epoch_decommitment_data/by_epoch/:epoch_id",
            get(handle_get_decommitment_data_by_epoch),
        )
        .route(
            "/get_epoch_decommitment_data/by_slot/:slot",
            get(handle_get_decommitment_data_by_slot),
        )
        .route(
            "/get_epoch_decommitment_data/by_execution_height/:execution_layer_height",
            get(handle_get_decommitment_data_by_execution_height),
        )
}
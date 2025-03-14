use axum::{routing::get, Router};
use crate::{handlers::decommitment::{handle_get_decommitment_data_by_epoch, handle_get_decommitment_data_by_execution_height, handle_get_decommitment_data_by_slot}, types::AppState};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/decommitment/by_epoch/:epoch_id",
            get(handle_get_decommitment_data_by_epoch),
        )
        .route(
            "/decommitment/by_slot/:slot",
            get(handle_get_decommitment_data_by_slot),
        )
        .route(
            "/decommitment/by_execution_height/:execution_height",
            get(handle_get_decommitment_data_by_execution_height),
        )
}

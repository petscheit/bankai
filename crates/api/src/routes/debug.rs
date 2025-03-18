use crate::handlers::debug::{
    handle_get_committee_hash, handle_get_epoch_proof,
    handle_get_latest_verified_committee, handle_get_latest_verified_slot, handle_get_status,
};
use crate::AppState;
use axum::{routing::get, Router};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(handle_get_status))
        .route(
            "/debug/get_latest_verified_epoch",
            get(handle_get_latest_verified_slot),
        )
        .route(
            "/debug/get_latest_verified_committee",
            get(handle_get_latest_verified_committee),
        )
        .route(
            "/get_verified_epoch_proof/:epoch",
            get(handle_get_epoch_proof),
        )
        .route(
            "/get_verified_committee_hash/:committee_id",
            get(handle_get_committee_hash),
        )
    // .route(

    // )
    // .route("/debug/get_job_status/:job_id", get(handle_get_job_status))
}

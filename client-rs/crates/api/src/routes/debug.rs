use crate::handlers::debug::{
    handle_get_latest_verified_slot,
    handle_get_latest_verified_committee,
    handle_get_job_status,
};
use axum::{routing::get, Router};
use thiserror::Error;

pub fn router() -> Router<AppState> {
    Router::new()
          .route(
            "/debug/get_latest_verified_epoch",
            get(handle_get_latest_verified_slot),
        )
        .route(
            "/debug/get_latest_verified_committee",
            get(handle_get_latest_verified_committee),
        )
        .route("/debug/get_job_status", get(handle_get_job_status))
}
use axum::{
    routing::{get, post},
    Router,
};
use thiserror::Error;

use crate::types::AppState;

// use crate::{handlers::task, AppState};
use crate::handlers::dashboard::handle_get_dashboard;

mod debug;
mod decommitment;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/dashboard",
            get(handle_get_dashboard),
        )
        .merge(debug::router())
        .merge(decommitment::router())
}

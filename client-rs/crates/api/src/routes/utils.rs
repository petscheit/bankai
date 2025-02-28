use crate::handlers::utils::{
    handle_get_status,
    handle_root_route,
};
use axum::{routing::get, Router};
use thiserror::Error;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handle_root_route),
        )
        .route(
            "/status",
            get(handle_get_status),
        )
}
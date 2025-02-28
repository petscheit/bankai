use crate::{helpers,
    state::{
        AppState, JobStatus
    }
};

use alloy_primitives::map::HashMap;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::Router,
    Json,
};
use std::{net::SocketAddr, time::Duration};
use tower::{ServiceBuilder, timeout::TimeoutLayer};
use tower_http::{limit::DefaultBodyLimit, trace::TraceLayer};
use tracing::{info, error};
use uuid::Uuid;
use num_traits::cast::ToPrimitive;
use serde_json::json;


#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
fn main() {
    let app = Router::new()
        .layer(DefaultBodyLimit::disable())
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http()), // Example: for logging/tracing
        )
        .layer((
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .with_state(app_state);

    let addr = "0.0.0.0:3001".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Bankai API server is listening on http://{}", addr);
}

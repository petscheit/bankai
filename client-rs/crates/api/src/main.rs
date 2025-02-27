

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
fn main() {
    let app = Router::new()
        .route("/", get(handle_root_route))
        .route("/status", get(handle_get_status))
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
        // ASCI-Art dashboard
        .route("/dashboard", get(handle_get_dashboard))
        // Some debug routes
        .route("/get_pending_atlantic_jobs", get(handle_get_epoch_proof))
        .route(
            "/get_verified_epoch_proof/:epoch",
            get(handle_get_epoch_proof),
        )
        .route(
            "/get_verified_committee_hash/:committee_id",
            get(handle_get_committee_hash),
        )
        .route(
            "/get_merkle_paths_for_epoch/:epoch_id",
            get(handle_get_merkle_paths_for_epoch),
        )
        // .route(
        //     "/debug/get_epoch_update/:slot",
        //     get(handle_get_epoch_update),
        // )
        .route(
            "/debug/get_latest_verified_epoch",
            get(handle_get_latest_verified_slot),
        )
        .route(
            "/debug/get_latest_verified_committee",
            get(handle_get_latest_verified_committee),
        )
        .route("/debug/get_job_status", get(handle_get_job_status))
        // .route("/get-merkle-inclusion-proof", get(handle_get_merkle_inclusion_proof))
        .layer(DefaultBodyLimit::disable())
        .layer(
            ServiceBuilder::new().layer(TraceLayer::new_for_http()), // Example: for logging/tracing
        )
        .layer((
            // Graceful shutdown will wait for outstanding requests to complete
            // Because of this timeourt setting, requests don't hang forever
            TimeoutLayer::new(Duration::from_secs(10)),
        ))
        .with_state(app_state);

    let addr = "0.0.0.0:3001".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Bankai RPC HTTP server is listening on http://{}", addr);
}

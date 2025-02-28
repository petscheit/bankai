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

pub async fn handle_get_job_status(
    Path(job_id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .db_manager
        .fetch_job_status(Uuid::parse_str(job_id.to_string().as_str()).unwrap())
        .await
    {
        Ok(Some(job_status)) => Json(json!({ "status": job_status.to_string()})),
        Ok(None) => Json(json!({ "error": "Job not found" })),
        Err(err) => {
            eprintln!("Failed to fetch job status: {:?}", err);
            Json(json!({ "error": "Failed to fetch job status" }))
        }
    }
}

pub async fn handle_get_latest_verified_committee(
    State(state): State<AppState>,
) -> impl IntoResponse {
    match state
        .bankai
        .starknet_client
        .get_latest_committee_id(&state.bankai.config)
        .await
    {
        Ok(latest_verified_committee) => {
            // Convert `Felt` to a string and parse it as a hexadecimal number
            let hex_string = latest_verified_committee.to_string(); // Ensure this converts to a "0x..." string
            match u64::from_str_radix(hex_string.trim_start_matches("0x"), 16) {
                Ok(committee_hash) => Json(json!({ "latest_verified_committee": committee_hash })),
                Err(err) => {
                    eprintln!(
                        "Failed to parse latest_verified_committee as decimal: {:?}",
                        err
                    );
                    Json(json!({ "error": "Invalid committee format" }))
                }
            }
        }
        Err(err) => {
            eprintln!("Failed to fetch latest epoch: {:?}", err);
            Json(json!({ "error": "Failed to fetch latest epoch" }))
        }
    }
}
//  RPC requests handling functions //
pub async fn handle_root_route(State(_state): State<AppState>) -> impl IntoResponse {
    Json(json!({ "success": true, "message": "Bankai daemon running" }))
}

// Handler for GET /status
pub async fn handle_get_status(State(state): State<AppState>) -> impl IntoResponse {
    let last_epoch_in_progress = match state.db_manager.get_latest_epoch_in_progress().await {
        Ok(Some(epoch)) => {
            let last_epoch_in_progress = epoch.to_u64().unwrap();
            last_epoch_in_progress
        }
        Ok(None) => 0,
        Err(_) => 0,
    };
    let in_progress_jobs_count = state.db_manager.count_jobs_in_progress().await.unwrap();
    let last_sync_committee_in_progress = state
        .db_manager
        .get_latest_sync_committee_in_progress()
        .await
        .unwrap()
        .unwrap();

    let errored_jobs = state
        .db_manager
        .get_jobs_with_statuses(vec![JobStatus::Error])
        .await
        .unwrap_or_default();

    let jobs_status_counts = state
        .db_manager
        .get_jobs_count_by_status()
        .await
        .unwrap_or_default();

    let mut jobs_status_map = HashMap::new();
    for job_status_count in jobs_status_counts {
        jobs_status_map.insert(job_status_count.status.to_string(), job_status_count.count);
    }

    Json(json!({ "success": true, "details": {
        "last_epoch_in_progress": last_epoch_in_progress,
        "last_sync_committee_in_progress": last_sync_committee_in_progress,
        "jobs_in_progress_count": in_progress_jobs_count,
        "jobs_statuses": jobs_status_map,
        "errored": errored_jobs
    } }))
}

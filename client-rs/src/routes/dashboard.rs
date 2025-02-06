use crate::{
    helpers,
    state::{AppState, JobStatus},
};
use axum::extract::State;
use num_traits::{SaturatingSub, ToPrimitive};

pub async fn handle_get_dashboard(State(state): State<AppState>) -> String {
    let db = state.db_manager.clone();
    let bankai = state.bankai.clone();

    // Fetch required stats
    let latest_beacon_slot = bankai.client.get_head_slot().await.unwrap_or_default();
    let latest_verified_slot = bankai
        .starknet_client
        .get_latest_epoch_slot(&bankai.config)
        .await
        .unwrap_or_default()
        .to_string()
        .parse::<u64>()
        .unwrap_or(0);

    // Calculate success rate from database
    let total_jobs = db.count_total_jobs().await.unwrap_or(0);
    let successful_jobs = db.count_successful_jobs().await.unwrap_or(0);
    let success_rate = if total_jobs > 0 {
        ((successful_jobs as f64 / total_jobs as f64) * 100.0).round()
    } else {
        0.0
    };

    // Calculate average job duration
    let avg_duration = db.get_average_job_duration().await.unwrap_or(0);
    let avg_duration_str = format!("{}s", avg_duration);

    let jobs_in_progress = db
        .count_jobs_in_progress()
        .await
        .unwrap_or(Some(0))
        .unwrap();

    // Fetch last 20 batch jobs
    let recent_batches = db.get_recent_batch_jobs(20).await.unwrap_or_default();

    // Format batch information
    let batch_info = recent_batches
        .iter()
        .map(|entry| {
            format!(
                "║  Batch {:}: {} -> {} [{}] {:<32} {:<66}  ║",
                entry.job.job_uuid.to_string()[..8].to_string(),
                entry.job.batch_range_begin_epoch,
                entry.job.batch_range_end_epoch,
                match entry.job.job_status {
                    JobStatus::Done => "✓",
                    JobStatus::Error => "✗",
                    _ => "⋯",
                },
                entry.job.job_status.to_string(),
                entry
                    .tx_hash
                    .as_ref()
                    .map_or("-".to_string(), |s| s.clone()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let batch_display = if recent_batches.is_empty() {
        "  ║  No recent batches found                                                ║    "
            .to_string()
    } else {
        batch_info
    };

    // Fetch last 20 batch jobs
    let recent_sync_committee_jobs = db
        .get_recent_sync_committee_jobs(20)
        .await
        .unwrap_or_default();

    // Format batch information
    let sync_committee_info = recent_sync_committee_jobs
        .iter()
        .map(|entry| {
            format!(
                "║  Batch {:}: {}  {}      [{}] {:<32} {:<66} ║",
                entry.job.job_uuid.to_string()[..8].to_string(),
                entry.job.slot,
                helpers::get_sync_committee_id_by_slot(entry.job.slot.to_u64().unwrap()),
                match entry.job.job_status {
                    JobStatus::Done => "✓",
                    JobStatus::Error => "✗",
                    _ => "⋯",
                },
                entry.job.job_status.to_string(),
                entry
                    .tx_hash
                    .as_ref()
                    .map_or("-".to_string(), |s| s.clone()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sync_committee_jobs_display = if recent_batches.is_empty() {
        "  ║  No recent sync committee jobs found                                                                                                                 ║    "
            .to_string()
    } else {
        sync_committee_info
    };

    // Update system health indicators with simpler checks
    let daemon_status = "● Active";
    let db_status = if db.is_connected().await {
        "● Connected"
    } else {
        "○ Disconnected"
    };
    let beacon_status = if bankai.client.get_head_slot().await.is_ok() {
        "● Connected"
    } else {
        "○ Disconnected"
    };

    let epoch_gap =
        (latest_beacon_slot.saturating_sub(latest_verified_slot) as f64 / 32.0).round() as u64;

    create_ascii_dashboard(
        latest_beacon_slot,
        latest_verified_slot,
        epoch_gap,
        success_rate,
        &avg_duration_str,
        jobs_in_progress,
        daemon_status,
        db_status,
        beacon_status,
        &batch_display,
        &sync_committee_jobs_display,
    )
}

pub fn create_ascii_dashboard(
    latest_beacon_slot: u64,
    latest_verified_slot: u64,
    epoch_gap: u64,
    success_rate: f64,
    avg_duration_str: &str,
    jobs_in_progress: u64,
    daemon_status: &str,
    db_status: &str,
    beacon_status: &str,
    batch_display: &str,
    sync_committee_jobs_display: &str,
) -> String {
    format!(
        r#"
 ____    _    _   _ _  __   _    ___
| __ )  / \  | \ | | |/ /  / \  |_ _|
|  _ \ / _ \ |  \| | ' /  / _ \  | |
| |_) / ___ \| |\  | . \ / ___ \ | |
|____/_/   \_\_| \_|_|\_/_/   \_\___|

╔════════════════════════════════════════ DASHBOARD OVERVIEW ════════════════════════════════════════════════════════════════════════════════╗
║                                                                                                                                            ║
║   Statuses:                                                                                                                                ║
║     • Daemon:    {daemon_status:<12}  • Database:  {db_status:<12}  • Beacon: {beacon_status:<12}                                                           ║
║                                                                                                                                            ║
║   Metrics:                                                                                                                                 ║
║     • Success Rate:        {success_rate:<10}                                                                                                      ║
║     • Average Duration:    {avg_duration:<10}                                                                                                      ║
║     • Jobs in Progress:    {jobs_in_progress:<10}                                                                                                      ║
║                                                                                                                                            ║
║   Beacon Info:                                                                                                                             ║
║     • Latest Beacon Slot:    {latest_beacon_slot:<12}                                                                                                  ║
║     • Latest Verified Slot:  {latest_verified_slot:<12}                                                                                                  ║
║     • Epoch Gap:             {epoch_gap:<12}                                                                                                  ║
║                                                                                                                                            ║
╠═══════════════════════════════════════ RECENT BATCH JOBS ══════════════════════════════════════════════════════════════════════════════════╣
║        UUID:     FROM:     TO:        STATUS:                          TX:                                                                 ║
║ ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────── ║
{batch_display_block}
╠══════════════════════════════   RECENT SYNC COMMITTEE JOBS  ═══════════════════════════════════════════════════════════════════════════════╣
║        UUID:     SLOT:    COMMITTEE:  STATUS:                          TX:                                                                 ║
║ ────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────── ║
{sync_committee_jobs_display_block}
╚════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝
"#,
        daemon_status = daemon_status,
        db_status = db_status,
        beacon_status = beacon_status,
        success_rate = format!("{:.2}%", success_rate),
        avg_duration = avg_duration_str,
        jobs_in_progress = jobs_in_progress,
        latest_beacon_slot = latest_beacon_slot,
        latest_verified_slot = latest_verified_slot,
        epoch_gap = epoch_gap,
        batch_display_block = batch_display,
        sync_committee_jobs_display_block = sync_committee_jobs_display
    )
}

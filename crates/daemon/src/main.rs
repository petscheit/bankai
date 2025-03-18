pub mod beacon_listener;
pub mod daemon;
pub mod error;
pub mod job_manager;
pub mod job_processor;
use daemon::Daemon;

// Main entry point
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), error::DaemonError> {
    // Initialize the daemon
    let mut daemon = Daemon::new().await?;

    // Run the daemon
    let _daemon_handle = tokio::spawn(async move {
        if let Err(e) = daemon.run().await {
            eprintln!("Daemon error: {:?}", e);
        }
    });

    // Create a channel to listen for shutdown signals
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Handle ctrl-c signal
    let _ctrl_c_handle = tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for ctrl-c: {:?}", e);
        }
        let _ = shutdown_tx.send(());
    });

    // Keep the main thread alive until shutdown signal
    let _ = shutdown_rx.await;

    println!("Shutting down daemon...");
    Ok(())
}

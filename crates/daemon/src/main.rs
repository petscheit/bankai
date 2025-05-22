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

    // Spawn a task to listen for shutdown signals
    let _shutdown_handle = tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{signal, SignalKind};
            // Create a stream for SIGTERM
            let mut sigterm =
                signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("Received Ctrl+C signal, shutting down...");
                },
                _ = sigterm.recv() => {
                    println!("Received SIGTERM signal, shutting down...");
                },
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, fallback to handling Ctrl+C only.
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to listen for Ctrl+C");
            println!("Received Ctrl+C signal, shutting down...");
        }
        // Send shutdown signal to the main thread
        let _ = shutdown_tx.send(());
    });

    // Keep the main thread alive until shutdown signal
    let _ = shutdown_rx.await;

    println!("Shutting down daemon...");
    Ok(())
}

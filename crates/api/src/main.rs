use std::{env, sync::Arc};

use bankai_core::{db::manager::DatabaseManager, BankaiClient};
use dotenv::from_filename;
use std::net::SocketAddr;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

mod handlers;
mod routes;
mod types;
mod utils;

use types::AppState;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load .env.sepolia file
    from_filename(".env.sepolia").ok();

    let subscriber = FmtSubscriber::builder()
        //.with_max_level(Level::DEBUG)
        .with_max_level(Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Validate environment variables
    let _ = utils::check_env_vars().map_err(|e| {
        error!("Error: {}", e);
        std::process::exit(1); // Exit if validation fails
    });

    //let (tx, mut rx) = mpsc::channel(32);
    let connection_string = format!(
        "host={} user={} password={} dbname={}",
        env::var("POSTGRES_HOST").unwrap().as_str(),
        env::var("POSTGRES_USER").unwrap().as_str(),
        env::var("POSTGRES_PASSWORD").unwrap().as_str(),
        env::var("POSTGRES_DB").unwrap().as_str()
    );

    // Create a new DatabaseManager
    let db_manager = Arc::new(DatabaseManager::new(&connection_string).await);
    let bankai = Arc::new(BankaiClient::new(false).await);

    let app_state: AppState = AppState {
        db_manager: db_manager.clone(),
        bankai: bankai.clone(),
    };

    let app = routes::router().with_state(app_state);

    let addr = "0.0.0.0:3001".parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("Bankai RPC HTTP server is listening on http://{}", addr);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
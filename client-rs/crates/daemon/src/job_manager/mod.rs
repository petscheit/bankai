use std::sync::Arc;

use bankai_core::{db::manager::DatabaseManager, types::job::Job, BankaiClient};
use tokio::sync::mpsc;

pub mod resume;
pub mod broadcast;
pub mod retry;

pub struct JobManager {
    db_manager: Arc<DatabaseManager>,
    bankai: Arc<BankaiClient>,
    tx: mpsc::Sender<Job>,
}

impl JobManager {
    pub fn new(db_manager: Arc<DatabaseManager>, bankai: Arc<BankaiClient>, tx: mpsc::Sender<Job>) -> Self {
        Self { db_manager, bankai, tx }
    }
}
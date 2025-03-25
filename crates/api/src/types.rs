use std::sync::Arc;

use bankai_core::{db::manager::DatabaseManager, BankaiClient};

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub bankai: Arc<BankaiClient>,
}

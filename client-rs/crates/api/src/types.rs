use std::sync::Arc;

use bankai_core::{BankaiClient, db::manager::DatabaseManager};

#[derive(Clone, Debug)]
pub struct AppState {
    pub db_manager: Arc<DatabaseManager>,
    pub bankai: Arc<BankaiClient>,
}

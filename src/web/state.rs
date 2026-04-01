use crate::config::Config;
use crate::model::SharedState;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: Pool<SqliteConnectionManager>,
    pub current_state: SharedState,
}

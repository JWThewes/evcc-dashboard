use crate::config::Config;
use crate::model::SharedState;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: Pool<SqliteConnectionManager>,
    pub current_state: SharedState,
    pub sse_tx: broadcast::Sender<SseEvent>,
}

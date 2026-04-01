use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::web::state::AppState;

pub async fn check(State(state): State<AppState>) -> Json<Value> {
    let db_ok = state.db_pool.get().is_ok();
    let has_data = state.current_state.read().await.last_updated.is_some();

    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "database": db_ok,
        "mqtt_receiving": has_data,
    }))
}

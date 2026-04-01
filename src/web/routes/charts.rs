use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;

use crate::db;
use crate::model::ChartData;
use crate::web::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChartQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_resolution")]
    pub resolution: String,
}

fn default_resolution() -> String {
    "auto".to_string()
}

pub async fn power_history(
    State(state): State<AppState>,
    Query(params): Query<ChartQuery>,
) -> Json<ChartData> {
    let now = Utc::now().timestamp();
    let from = params.from.unwrap_or(now - 86400); // default: last 24h
    let to = params.to.unwrap_or(now);
    let resolution = resolve_auto_resolution(&params.resolution, from, to);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_power_history(&conn, from, to, &resolution)
    })
    .await
    .unwrap_or_else(|_| Ok(empty_chart_data()))
    .unwrap_or_else(|_| empty_chart_data());

    Json(data)
}

pub async fn energy_daily(
    State(state): State<AppState>,
    Query(params): Query<ChartQuery>,
) -> Json<ChartData> {
    let now = Utc::now().timestamp();
    let from = params.from.unwrap_or(now - 30 * 86400); // default: last 30 days
    let to = params.to.unwrap_or(now);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_daily_chart(&conn, from, to)
    })
    .await
    .unwrap_or_else(|_| Ok(empty_chart_data()))
    .unwrap_or_else(|_| empty_chart_data());

    Json(data)
}

pub async fn battery_history(
    State(state): State<AppState>,
    Query(params): Query<ChartQuery>,
) -> Json<ChartData> {
    let now = Utc::now().timestamp();
    let from = params.from.unwrap_or(now - 86400);
    let to = params.to.unwrap_or(now);
    let resolution = resolve_auto_resolution(&params.resolution, from, to);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_battery_history(&conn, from, to, &resolution)
    })
    .await
    .unwrap_or_else(|_| Ok(empty_chart_data()))
    .unwrap_or_else(|_| empty_chart_data());

    Json(data)
}

pub async fn loadpoint_history(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<u32>,
    Query(params): Query<ChartQuery>,
) -> Json<ChartData> {
    let now = Utc::now().timestamp();
    let from = params.from.unwrap_or(now - 86400);
    let to = params.to.unwrap_or(now);
    let resolution = resolve_auto_resolution(&params.resolution, from, to);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_loadpoint_history(&conn, id, from, to, &resolution)
    })
    .await
    .unwrap_or_else(|_| Ok(empty_chart_data()))
    .unwrap_or_else(|_| empty_chart_data());

    Json(data)
}

fn resolve_auto_resolution(resolution: &str, from: i64, to: i64) -> String {
    if resolution != "auto" {
        return resolution.to_string();
    }
    let range = to - from;
    if range <= 3600 {
        "raw".to_string() // <=1h: 5s resolution
    } else if range <= 86400 {
        "1m".to_string() // <=1d: 1min resolution
    } else if range <= 7 * 86400 {
        "5m".to_string() // <=1w: 5min resolution
    } else {
        "1h".to_string() // >1w: hourly resolution
    }
}

fn empty_chart_data() -> ChartData {
    ChartData {
        timestamps: vec![],
        series: HashMap::new(),
    }
}

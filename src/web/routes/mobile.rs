use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use chrono::Utc;
use futures_util::stream::Stream;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::db;
use crate::model::{ChartData, CurrentState, DailySummary, LoadpointState};
use crate::web::state::AppState;

#[derive(Debug, Serialize)]
struct LiveEvent {
    #[serde(flatten)]
    state: CurrentState,
    pv_energy_today_wh: Option<f64>,
}

pub async fn live(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let interval = tokio::time::interval(Duration::from_secs(5));
    let stream = IntervalStream::new(interval).map(move |_| {
        let state = state.clone();
        async move {
            let current = state.current_state.read().await.clone();

            let pv_energy_today_wh = {
                let pool = state.db_pool.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get().ok()?;
                    db::query::query_today_pv_energy(&conn).ok()?
                })
                .await
                .ok()
                .flatten()
            };

            let event = LiveEvent {
                state: current,
                pv_energy_today_wh,
            };

            Ok::<_, Infallible>(
                Event::default()
                    .json_data(&event)
                    .unwrap_or_else(|_| Event::default().data("{}")),
            )
        }
    });

    let stream = stream.then(|fut| fut);

    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn current_state(State(state): State<AppState>) -> Json<CurrentState> {
    let current = state.current_state.read().await.clone();
    Json(current)
}

#[derive(Debug, Deserialize)]
pub struct EnergyQuery {
    pub from: Option<i64>,
    pub to: Option<i64>,
    #[serde(default = "default_resolution")]
    pub resolution: String,
}

fn default_resolution() -> String {
    "auto".to_string()
}

fn resolve_auto_resolution(resolution: &str, from: i64, to: i64) -> String {
    if resolution != "auto" {
        return resolution.to_string();
    }
    let range = to - from;
    if range <= 3600 {
        "raw".to_string()
    } else if range <= 86400 {
        "1m".to_string()
    } else if range <= 7 * 86400 {
        "5m".to_string()
    } else {
        "1h".to_string()
    }
}

pub async fn energy_history(
    State(state): State<AppState>,
    Query(params): Query<EnergyQuery>,
) -> Json<ChartData> {
    let now = Utc::now().timestamp();
    let from = params.from.unwrap_or(now - 86400);
    let to = params.to.unwrap_or(now);
    let resolution = resolve_auto_resolution(&params.resolution, from, to);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_power_history(&conn, from, to, &resolution)
    })
    .await
    .unwrap_or_else(|_| Ok(empty_chart()))
    .unwrap_or_else(|_| empty_chart());

    Json(data)
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}

pub async fn daily_summaries(
    State(state): State<AppState>,
    Query(params): Query<SummaryQuery>,
) -> Json<Vec<DailySummary>> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let from = params.from.unwrap_or_else(|| today.clone());
    let to = params.to.unwrap_or(today);

    let pool = state.db_pool.clone();
    let data = tokio::task::spawn_blocking(move || {
        let conn = pool.get().expect("db connection");
        db::query::query_daily_summaries(&conn, &from, &to)
    })
    .await
    .unwrap_or_else(|_| Ok(vec![]))
    .unwrap_or_default();

    Json(data)
}

pub async fn loadpoints_list(State(state): State<AppState>) -> Json<Vec<LoadpointState>> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);
    Json(loadpoints)
}

pub async fn loadpoint_detail(
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> Result<Json<LoadpointState>, axum::http::StatusCode> {
    let current = state.current_state.read().await;
    current
        .loadpoints
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(axum::http::StatusCode::NOT_FOUND)
}

fn empty_chart() -> ChartData {
    ChartData {
        timestamps: vec![],
        series: std::collections::HashMap::new(),
    }
}

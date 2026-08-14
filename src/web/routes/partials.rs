use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::db;
use crate::model::{EnergyTotals, LoadpointState, SiteState};
use crate::web::state::AppState;

// ---------- Legacy partial endpoints (backward-compatible) ----------

#[derive(Template)]
#[template(path = "partials/energy_flow.html")]
pub struct EnergyFlowTemplate {
    pub site: SiteState,
}

pub async fn energy_flow(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let tmpl = EnergyFlowTemplate {
        site: current.site.clone(),
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/loadpoints.html")]
pub struct LoadpointsTemplate {
    pub loadpoints: Vec<LoadpointState>,
}

pub async fn loadpoints(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);
    let tmpl = LoadpointsTemplate { loadpoints };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/battery_status.html")]
pub struct BatteryStatusTemplate {
    pub site: SiteState,
}

pub async fn battery_status(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let tmpl = BatteryStatusTemplate {
        site: current.site.clone(),
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/summary_stats.html")]
pub struct SummaryStatsTemplate {
    pub site: SiteState,
}

pub async fn summary_stats(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let tmpl = SummaryStatsTemplate {
        site: current.site.clone(),
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/today_energy.html")]
pub struct TodayEnergyTemplate {
    pub totals: EnergyTotals,
    pub green_share: Option<f64>,
}

pub async fn today_energy(State(state): State<AppState>) -> Html<String> {
    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;
    let green_share = state.current_state.read().await.site.green_share_home;

    let totals = tokio::task::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let now_ts = now.timestamp();
        match pool.get() {
            Ok(conn) => db::query::query_energy_totals(&conn, today_start, now_ts, interval)
                .unwrap_or_default(),
            Err(_) => EnergyTotals::default(),
        }
    })
    .await
    .unwrap_or_default();

    let tmpl = TodayEnergyTemplate { totals, green_share };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

// ---------- New three-zone partial endpoints ----------

#[derive(Template)]
#[template(path = "partials/schematic.html")]
pub struct SchematicPartialTemplate {
    pub site: SiteState,
}

/// Renders the hero zone schematic placeholder partial.
pub async fn schematic(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let tmpl = SchematicPartialTemplate {
        site: current.site.clone(),
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/cards.html")]
pub struct CardsPartialTemplate {
    pub site: SiteState,
    pub loadpoints: Vec<LoadpointState>,
}

/// Renders the cards zone partial with energy flow, battery, and loadpoints.
pub async fn cards(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);
    let tmpl = CardsPartialTemplate {
        site: current.site.clone(),
        loadpoints,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "partials/summary_new.html")]
pub struct SummaryNewPartialTemplate {
    pub totals: EnergyTotals,
    pub green_share: Option<f64>,
}

/// Renders the consolidated summary zone partial with today's energy totals.
pub async fn summary_new(State(state): State<AppState>) -> Html<String> {
    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;
    let green_share = state.current_state.read().await.site.green_share_home;

    let totals = tokio::task::spawn_blocking(move || {
        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let now_ts = now.timestamp();
        match pool.get() {
            Ok(conn) => db::query::query_energy_totals(&conn, today_start, now_ts, interval)
                .unwrap_or_default(),
            Err(_) => EnergyTotals::default(),
        }
    })
    .await
    .unwrap_or_default();

    let tmpl = SummaryNewPartialTemplate { totals, green_share };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

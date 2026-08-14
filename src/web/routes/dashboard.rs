use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::db;
use crate::model::{EnergyTotals, LoadpointState, SiteState};
use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub base_path: String,
    pub site: SiteState,
    pub loadpoints: Vec<LoadpointState>,
    pub totals: EnergyTotals,
    pub green_share: Option<f64>,
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);
    let green_share = current.site.green_share_home;
    let site = current.site.clone();
    drop(current); // Release read lock before blocking

    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;

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

    let tmpl = DashboardTemplate {
        base_path: state.config.server.base_path.clone(),
        site,
        loadpoints,
        totals,
        green_share,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "history.html")]
pub struct HistoryTemplate {
    pub base_path: String,
    pub today: String,
    pub min_date: String,
}

pub async fn history(State(state): State<AppState>) -> Html<String> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let min_date = {
        let pool = state.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().ok()?;
            db::query::query_earliest_timestamp(&conn)
        })
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| {
            // Default to 90 days ago if no data
            (chrono::Local::now() - chrono::Duration::days(90))
                .format("%Y-%m-%d")
                .to_string()
        })
    };

    let tmpl = HistoryTemplate {
        base_path: state.config.server.base_path.clone(),
        today,
        min_date,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

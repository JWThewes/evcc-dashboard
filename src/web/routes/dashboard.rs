use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::db;
use crate::model::{EnergyTotals, LoadpointState, SiteState};
use crate::web::state::{
    AppState, FlowDirection, SchematicNodeState,
    compute_intensity, flow_direction, node_visible,
    derive_schematic_aria_label, derive_status_announcement,
};

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub base_path: String,
    pub site: SiteState,
    pub loadpoints: Vec<LoadpointState>,
    pub totals: EnergyTotals,
    pub green_share: Option<f64>,
    pub nodes: Vec<SchematicNodeState>,
    pub stale: bool,
    pub aria_label: String,
    pub status_announcement: Option<String>,
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let now = chrono::Utc::now().timestamp();

    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);
    let green_share = current.site.green_share_home;
    let site = current.site.clone();

    // Read peak cache for schematic
    let peaks = state
        .peak_cache
        .read()
        .map(|p| p.clone())
        .unwrap_or_default();

    // Determine staleness
    let stale = match current.last_updated {
        Some(ts) => (now - ts) > 30,
        None => true,
    };

    // Compute schematic nodes
    let nodes = vec![
        SchematicNodeState {
            id: "house",
            label: "Home",
            visible: true,
            current_watts: current.site.home_power,
            intensity: 0.0,
            direction: FlowDirection::Idle,
            cx: 200.0,
            cy: 200.0,
        },
        {
            let watts = current.site.pv_power;
            let visible = node_visible(current.site.pv_last_seen, now);
            let intensity = compute_intensity(watts.unwrap_or(0.0), peaks.pv_peak);
            SchematicNodeState {
                id: "solar",
                label: "Solar",
                visible,
                current_watts: watts,
                intensity,
                direction: flow_direction("solar", watts),
                cx: 200.0,
                cy: 60.0,
            }
        },
        {
            let watts = current.site.grid_power;
            let visible = node_visible(current.site.grid_last_seen, now);
            let intensity = compute_intensity(watts.unwrap_or(0.0), peaks.grid_peak);
            SchematicNodeState {
                id: "grid",
                label: "Grid",
                visible,
                current_watts: watts,
                intensity,
                direction: flow_direction("grid", watts),
                cx: 340.0,
                cy: 200.0,
            }
        },
        {
            let watts = current.site.battery_power;
            let visible = node_visible(current.site.battery_last_seen, now);
            let intensity = compute_intensity(watts.unwrap_or(0.0), peaks.battery_peak);
            SchematicNodeState {
                id: "battery",
                label: "Battery",
                visible,
                current_watts: watts,
                intensity,
                direction: flow_direction("battery", watts),
                cx: 310.0,
                cy: 340.0,
            }
        },
        {
            let ev_watts: Option<f64> = {
                let total: f64 = current
                    .loadpoints
                    .values()
                    .filter(|lp| lp.connected.unwrap_or(false))
                    .filter_map(|lp| lp.charge_power)
                    .sum();
                if total > 0.0 { Some(total) } else { None }
            };
            let visible = node_visible(current.ev_last_seen, now);
            let intensity = compute_intensity(ev_watts.unwrap_or(0.0), peaks.home_peak);
            SchematicNodeState {
                id: "ev",
                label: "EV",
                visible,
                current_watts: ev_watts,
                intensity,
                direction: flow_direction("ev", ev_watts),
                cx: 90.0,
                cy: 340.0,
            }
        },
    ];

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

    // Compute ARIA accessibility context for initial SSR
    let aria_label = derive_schematic_aria_label(&nodes);
    let status_announcement = {
        let previous = state
            .last_announcement
            .read()
            .map(|s| s.clone())
            .unwrap_or_default();
        let announcement = derive_status_announcement(&nodes, stale, &previous);
        if let Some(ref text) = announcement {
            if let Ok(mut prev) = state.last_announcement.write() {
                *prev = text.clone();
            }
        }
        announcement
    };

    let tmpl = DashboardTemplate {
        base_path: state.config.server.base_path.clone(),
        site,
        loadpoints,
        totals,
        green_share,
        nodes,
        stale,
        aria_label,
        status_announcement,
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

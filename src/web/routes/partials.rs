use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::db;
use crate::model::{EnergyTotals, LoadpointState, SiteState};
use crate::web::state::{
    AppState, FlowDirection, SchematicNodeState, SchematicRenderContext,
    compute_intensity, flow_direction, node_visible,
    derive_schematic_aria_label, derive_status_announcement,
};

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

/// Renders the hero zone SVG energy-flow schematic with animated flow paths.
/// Reads from in-memory state only — no database access on request path.
pub async fn schematic(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let now = chrono::Utc::now().timestamp();

    // Read peak cache (std::sync::RwLock — non-async, sub-microsecond)
    let peaks = state
        .peak_cache
        .read()
        .map(|p| p.clone())
        .unwrap_or_default();

    // Determine overall staleness (no MQTT data in last 30 seconds)
    let stale = match current.last_updated {
        Some(ts) => (now - ts) > 30,
        None => true,
    };

    // Radial layout positions (400×400 viewBox, house at center)
    // Solar: top, Grid: right, Battery: bottom-right, EV: bottom-left
    let nodes = vec![
        // House node (always visible, no flow path)
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
        // Solar node
        {
            let watts = current.site.pv_power;
            let visible = node_visible(current.site.pv_last_seen, now);
            let intensity = compute_intensity(
                watts.unwrap_or(0.0),
                peaks.pv_peak,
            );
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
        // Grid node
        {
            let watts = current.site.grid_power;
            let visible = node_visible(current.site.grid_last_seen, now);
            let intensity = compute_intensity(
                watts.unwrap_or(0.0),
                peaks.grid_peak,
            );
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
        // Battery node
        {
            let watts = current.site.battery_power;
            let visible = node_visible(current.site.battery_last_seen, now);
            let intensity = compute_intensity(
                watts.unwrap_or(0.0),
                peaks.battery_peak,
            );
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
        // EV node
        {
            // EV power is the sum of all connected loadpoint charge powers
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
            // Use home_peak as proxy for EV peak since we don't track EV separately
            let intensity = compute_intensity(
                ev_watts.unwrap_or(0.0),
                peaks.home_peak,
            );
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

    // Compute dynamic ARIA label for schematic SVG
    let aria_label = derive_schematic_aria_label(&nodes);

    // Compute status announcement with deduplication
    let status_announcement = {
        let previous = state
            .last_announcement
            .read()
            .map(|s| s.clone())
            .unwrap_or_default();
        let announcement = derive_status_announcement(&nodes, stale, &previous);
        // Update previous announcement state if changed
        if let Some(ref text) = announcement {
            if let Ok(mut prev) = state.last_announcement.write() {
                *prev = text.clone();
            }
        }
        announcement
    };

    let tmpl = SchematicRenderContext { nodes, stale, aria_label, status_announcement };
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

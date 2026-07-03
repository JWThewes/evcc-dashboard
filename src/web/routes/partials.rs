use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::db;
use crate::model::{EnergyTotals, LoadpointState, SiteState};
use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "partials/energy_flow.html")]
pub struct EnergyFlowTemplate {
    pub site: SiteState,
    pub loadpoint_power: f64,
    pub loadpoint_charging: bool,
    pub base_path: String,
}

impl EnergyFlowTemplate {
    pub fn loadpoint_power_kw(&self) -> String {
        if self.loadpoint_power >= 1000.0 {
            format!("{:.1} kW", self.loadpoint_power / 1000.0)
        } else if self.loadpoint_power > 0.0 {
            format!("{:.0} W", self.loadpoint_power)
        } else {
            "0 W".to_string()
        }
    }

    pub fn loadpoint_speed_class(&self) -> &str {
        if self.loadpoint_power > 5000.0 { "flow-speed-max" }
        else if self.loadpoint_power > 2000.0 { "flow-speed-high" }
        else if self.loadpoint_power > 500.0 { "flow-speed-medium" }
        else if self.loadpoint_power > 0.0 { "flow-speed-low" }
        else { "" }
    }

    pub fn car_status_label(&self) -> &str {
        if self.loadpoint_charging { "Charging" } else { "Idle" }
    }

    pub fn pv_speed_class(&self) -> &'static str {
        SiteState::flow_speed_class(self.site.pv_power)
    }

    pub fn grid_speed_class(&self) -> &'static str {
        SiteState::flow_speed_class(self.site.grid_power)
    }

    pub fn battery_speed_class(&self) -> &'static str {
        SiteState::flow_speed_class(self.site.battery_power)
    }

    pub fn aria_description(&self) -> String {
        let pv = self.site.pv_power.map_or("PV unavailable".to_string(), |v| {
            if v > 0.0 {
                format!("PV producing {}", SiteState::format_kw(v))
            } else {
                "PV idle".to_string()
            }
        });
        let home = self.site.home_power.map_or("Home unavailable".to_string(), |v| {
            format!("Home consuming {}", SiteState::format_kw(v))
        });
        let grid = match self.site.grid_power {
            Some(v) if v > 0.0 => format!("Importing {} from grid", SiteState::format_kw(v)),
            Some(v) if v < 0.0 => format!("Exporting {} to grid", SiteState::format_kw(v.abs())),
            Some(_) => "Grid idle".to_string(),
            None => "Grid unavailable".to_string(),
        };
        let battery = match (self.site.battery_power, self.site.battery_soc) {
            (Some(p), Some(soc)) if p < 0.0 => format!("Battery charging at {}, {:.0} percent", SiteState::format_kw(p.abs()), soc),
            (Some(p), Some(soc)) if p > 0.0 => format!("Battery discharging at {}, {:.0} percent", SiteState::format_kw(p), soc),
            (Some(_), Some(soc)) => format!("Battery idle, {:.0} percent", soc),
            _ => "Battery unavailable".to_string(),
        };
        let car = if self.loadpoint_charging {
            format!("EV charging at {}", self.loadpoint_power_kw())
        } else {
            "EV idle".to_string()
        };
        format!("Energy flow: {}. {}. {}. {}. {}.", pv, home, grid, battery, car)
    }
}

pub async fn energy_flow(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;

    let loadpoint_power: f64 = current.loadpoints.values()
        .filter_map(|lp| lp.charge_power)
        .sum();

    let tmpl = EnergyFlowTemplate {
        site: current.site.clone(),
        loadpoint_power,
        loadpoint_charging: loadpoint_power > 0.0,
        base_path: state.config.server.base_path.clone().unwrap_or_default(),
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

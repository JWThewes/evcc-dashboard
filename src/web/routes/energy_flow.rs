//! Energy flow visualization route handler.
//!
//! Provides the `/partials/energy-flow-svg` endpoint serving OOB swap fragments
//! for live energy flow updates every 3 seconds. This unit owns
//! contract-oob-swap-ids (14 stable element IDs).

use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::model::SiteState;
use crate::web::state::AppState;

// ---------------------------------------------------------------------------
// Domain: Energy Flow State (derived view model)
// ---------------------------------------------------------------------------

/// Noise floor threshold in watts — values below this are treated as zero.
const NOISE_FLOOR_W: f64 = 10.0;

/// Minimum particle animation duration in ms (at max power ~10kW).
const ANIM_DURATION_MIN_MS: u32 = 1500;
/// Maximum particle animation duration in ms (at noise floor ~50W).
const ANIM_DURATION_MAX_MS: u32 = 6000;
/// Power value mapped to minimum duration.
const POWER_MAX_W: f64 = 10000.0;

/// Direction of energy flow on a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    Forward,
    Reverse,
    Idle,
}

impl FlowDirection {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Forward => "ef-dir-forward",
            Self::Reverse => "ef-dir-reverse",
            Self::Idle => "ef-dir-idle",
        }
    }
}

/// A flow path between two nodes.
#[derive(Debug, Clone)]
pub struct FlowPath {
    pub id: &'static str,
    pub power: f64,
    pub direction: FlowDirection,
    pub active: bool,
    pub duration_ms: u32,
}

impl FlowPath {
    /// Whether this path is flowing in reverse direction (for template conditionals).
    pub fn is_reverse(&self) -> bool {
        self.direction == FlowDirection::Reverse
    }

    /// Integer power for template display.
    pub fn power_display(&self) -> i32 {
        self.power as i32
    }
}

/// Complete energy flow state derived from SharedState.
#[derive(Debug, Clone)]
pub struct EnergyFlowState {
    pub pv_power: f64,
    pub grid_power: f64,
    pub home_power: f64,
    pub battery_power: f64,
    pub ev_power: f64,
    pub self_sufficiency: f64,
    pub pv_path: FlowPath,
    pub grid_path: FlowPath,
    pub battery_path: FlowPath,
    pub ev_path: FlowPath,
    pub has_battery: bool,
    pub has_ev: bool,
}

impl EnergyFlowState {
    // Display helpers for templates (Askama cannot do `as i32` directly)
    pub fn pv_power_display(&self) -> i32 { self.pv_power as i32 }
    pub fn grid_power_abs_display(&self) -> i32 { self.grid_power.abs() as i32 }
    pub fn home_power_display(&self) -> i32 { self.home_power as i32 }
    pub fn battery_power_abs_display(&self) -> i32 { self.battery_power.abs() as i32 }
    pub fn ev_power_display(&self) -> i32 { self.ev_power as i32 }
    pub fn self_sufficiency_display(&self) -> i32 { self.self_sufficiency as i32 }
}

impl EnergyFlowState {
    /// Build from current site state.
    pub fn from_site(site: &SiteState) -> Self {
        let pv = site.pv_power.unwrap_or(0.0);
        let grid = site.grid_power.unwrap_or(0.0);
        let home = site.home_power.unwrap_or(0.0);
        let battery = site.battery_power.unwrap_or(0.0);
        let ev = site.ev_power.unwrap_or(0.0);

        // Self-sufficiency: (home - grid_import) / home * 100, clamped 0-100
        let grid_import = grid.max(0.0);
        let self_sufficiency = if home > NOISE_FLOOR_W {
            ((home - grid_import) / home * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        // PV path: always forward (PV -> Home), idle if below noise floor
        let pv_path = Self::build_path("pv", pv, true);
        // Grid path: positive = import (Grid -> Home), negative = export (Home -> Grid)
        let grid_path = Self::build_path_bidirectional("grid", grid);
        // Battery: positive = discharging (Battery -> Home), negative = charging (Home -> Battery)
        let battery_path = Self::build_path_bidirectional("battery", battery);
        // EV path: always forward (Home -> EV consumption), idle if no EV
        let ev_path = Self::build_path("ev", ev, true);

        let has_battery = site.battery_soc.is_some() || battery.abs() > NOISE_FLOOR_W;
        let has_ev = site.ev_connected || ev > NOISE_FLOOR_W;

        Self {
            pv_power: pv,
            grid_power: grid,
            home_power: home,
            battery_power: battery,
            ev_power: ev,
            self_sufficiency,
            pv_path,
            grid_path,
            battery_path,
            ev_path,
            has_battery,
            has_ev,
        }
    }

    /// Build a unidirectional path (PV, EV).
    fn build_path(id: &'static str, power: f64, _forward: bool) -> FlowPath {
        let active = power.abs() > NOISE_FLOOR_W;
        FlowPath {
            id,
            power: power.abs(),
            direction: if active {
                FlowDirection::Forward
            } else {
                FlowDirection::Idle
            },
            active,
            duration_ms: Self::calc_duration(power.abs()),
        }
    }

    /// Build a bidirectional path (Grid, Battery).
    /// Positive = forward (toward home), Negative = reverse (away from home).
    fn build_path_bidirectional(id: &'static str, power: f64) -> FlowPath {
        let abs_power = power.abs();
        let active = abs_power > NOISE_FLOOR_W;
        let direction = if !active {
            FlowDirection::Idle
        } else if power > 0.0 {
            FlowDirection::Forward
        } else {
            FlowDirection::Reverse
        };
        FlowPath {
            id,
            power: abs_power,
            direction,
            active,
            duration_ms: Self::calc_duration(abs_power),
        }
    }

    /// Calculate animation duration: inverse linear from 1500ms (10kW) to 6000ms (50W).
    fn calc_duration(power: f64) -> u32 {
        if power <= NOISE_FLOOR_W {
            return ANIM_DURATION_MAX_MS;
        }
        let clamped = power.min(POWER_MAX_W);
        let ratio = (clamped - NOISE_FLOOR_W) / (POWER_MAX_W - NOISE_FLOOR_W);
        let duration = ANIM_DURATION_MAX_MS as f64
            - ratio * (ANIM_DURATION_MAX_MS - ANIM_DURATION_MIN_MS) as f64;
        duration as u32
    }

    /// Format power for display with W/kW unit.
    pub fn format_power(watts: f64) -> String {
        if watts.abs() >= 1000.0 {
            format!("{:.1} kW", watts / 1000.0)
        } else {
            format!("{:.0} W", watts)
        }
    }
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

/// Full energy flow SVG template (initial server render).
#[derive(Template)]
#[template(path = "partials/energy_flow_svg.html")]
pub struct EnergyFlowSvgTemplate<'a> {
    pub state: EnergyFlowState,
    pub base_path: &'a str,
}

/// OOB swap fragment for polling updates.
#[derive(Template)]
#[template(path = "partials/energy_flow_oob.html")]
pub struct EnergyFlowOobTemplate {
    pub state: EnergyFlowState,
}

// ---------------------------------------------------------------------------
// Route Handler
// ---------------------------------------------------------------------------

/// Polling endpoint: returns OOB swap fragments for live energy flow updates.
/// Protected by cookie auth middleware (registered in router).
pub async fn energy_flow_partial(State(app_state): State<AppState>) -> Html<String> {
    let current = app_state.current_state.read().await;
    let flow_state = EnergyFlowState::from_site(&current.site);
    drop(current); // Release lock immediately

    let tmpl = EnergyFlowOobTemplate { state: flow_state };
    Html(tmpl.render().unwrap_or_else(|e| format!("<!-- render error: {e} -->")))
}

/// Full SVG render for initial page load (embedded in overview partial).
pub fn render_energy_flow_svg(site: &SiteState, base_path: &str) -> String {
    let flow_state = EnergyFlowState::from_site(site);
    let tmpl = EnergyFlowSvgTemplate {
        state: flow_state,
        base_path,
    };
    tmpl.render().unwrap_or_else(|e| format!("<!-- render error: {e} -->"))
}

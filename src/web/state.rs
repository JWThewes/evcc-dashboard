use askama::Template;
use crate::config::Config;
use crate::model::SharedState;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

pub type SharedPeakCache = Arc<std::sync::RwLock<PeakCache>>;

/// Cached 7-day peak absolute power values per energy path.
/// Updated every 5 minutes by the peak updater background task.
#[derive(Debug, Clone, Default)]
pub struct PeakCache {
    /// Peak absolute grid power (watts) over rolling 7 days
    pub grid_peak: f64,
    /// Peak absolute PV power (watts) over rolling 7 days
    pub pv_peak: f64,
    /// Peak absolute battery power (watts) over rolling 7 days
    pub battery_peak: f64,
    /// Peak absolute home consumption (watts) over rolling 7 days
    pub home_peak: f64,
    /// Timestamp of last successful refresh (Unix epoch seconds)
    pub last_refreshed: Option<i64>,
}

/// Direction of energy flow on a schematic path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlowDirection {
    /// Energy flowing toward House (import/consumption)
    ToHouse,
    /// Energy flowing away from House (export/production return)
    FromHouse,
    /// No flow (idle — below 50W threshold or node invisible)
    Idle,
}

impl FlowDirection {
    /// Convert to data-direction attribute value
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::ToHouse => "import",
            Self::FromHouse => "export",
            Self::Idle => "idle",
        }
    }
}

/// Per-node computed state for a single schematic render cycle.
#[derive(Debug, Clone)]
pub struct SchematicNodeState {
    /// Node identifier (solar, grid, battery, ev, house)
    pub id: &'static str,
    /// Display label
    pub label: &'static str,
    /// Whether the node should be rendered visible
    pub visible: bool,
    /// Current absolute power in watts (None if no data)
    pub current_watts: Option<f64>,
    /// Animation intensity [0.0, 1.0] for the connecting path
    pub intensity: f64,
    /// Flow direction relative to House
    pub direction: FlowDirection,
    /// SVG position x in viewBox coordinates
    pub cx: f64,
    /// SVG position y in viewBox coordinates
    pub cy: f64,
}

impl SchematicNodeState {
    /// Format intensity for the data attribute (2 decimal places)
    pub fn intensity_str(&self) -> String {
        format!("{:.2}", self.intensity)
    }

    /// Format watts for display
    pub fn watts_display(&self) -> String {
        match self.current_watts {
            Some(w) => format!("{:.0}", w.abs()),
            None => "—".to_string(),
        }
    }

    /// Return raw watts value for data-watts attribute
    pub fn watts_attr(&self) -> String {
        match self.current_watts {
            Some(w) => format!("{:.0}", w.abs()),
            None => "0".to_string(),
        }
    }
}

/// Complete render context for the schematic Askama template.
#[derive(Template)]
#[template(path = "partials/schematic.html")]
pub struct SchematicRenderContext {
    /// Per-node computed states
    pub nodes: Vec<SchematicNodeState>,
    /// Whether the overall state is stale (no recent MQTT data)
    pub stale: bool,
}

/// Compute animation intensity from current watts and 7-day peak.
pub fn compute_intensity(current_watts: f64, peak_7d_watts: f64) -> f64 {
    // Below idle threshold → no animation
    if current_watts.abs() < 50.0 {
        return 0.0;
    }

    // Fresh install fallback: peak is zero
    if peak_7d_watts <= 0.0 {
        return 1.0;
    }

    // Normal case: proportional to peak
    let ratio = current_watts.abs() / peak_7d_watts;
    ratio.clamp(0.0, 1.0)
}

/// Determine node visibility based on last_seen timestamp.
/// Returns true if data was received within the last 60 seconds.
pub fn node_visible(last_seen: Option<i64>, now: i64) -> bool {
    match last_seen {
        None => false,
        Some(ts) => (now - ts) < 60,
    }
}

/// Determine flow direction for a given node based on power sign conventions.
pub fn flow_direction(node_id: &str, watts: Option<f64>) -> FlowDirection {
    match watts {
        None => FlowDirection::Idle,
        Some(w) if w.abs() < 50.0 => FlowDirection::Idle,
        Some(w) => match node_id {
            "solar" => FlowDirection::ToHouse, // Solar always flows to house
            "grid" => {
                if w > 0.0 {
                    FlowDirection::ToHouse // Importing from grid
                } else {
                    FlowDirection::FromHouse // Exporting to grid
                }
            }
            "battery" => {
                // evcc: positive = discharging (to house), negative = charging (from house)
                if w > 0.0 {
                    FlowDirection::ToHouse
                } else {
                    FlowDirection::FromHouse
                }
            }
            "ev" => FlowDirection::FromHouse, // EV only consumes (from house)
            _ => FlowDirection::Idle,
        },
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db_pool: Pool<SqliteConnectionManager>,
    pub current_state: SharedState,
    pub peak_cache: SharedPeakCache,
}

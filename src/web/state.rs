use askama::Template;
use crate::config::Config;
use crate::model::SharedState;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;

pub type SharedPeakCache = Arc<std::sync::RwLock<PeakCache>>;

/// Previous ARIA announcement text for deduplication.
/// Only announces when state changes between poll cycles.
pub type SharedAnnouncementState = Arc<std::sync::RwLock<String>>;

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
    /// Dynamic aria-label describing current energy flow state
    pub aria_label: String,
    /// Status announcement text (Some only when state changed from previous poll)
    pub status_announcement: Option<String>,
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
    pub last_announcement: SharedAnnouncementState,
}

/// Derive a human-readable aria-label for the schematic SVG element.
/// Summarises current energy flow state for screen readers.
pub fn derive_schematic_aria_label(nodes: &[SchematicNodeState]) -> String {
    let mut parts = Vec::new();

    for node in nodes {
        if node.id == "house" {
            continue;
        }
        if !node.visible {
            continue;
        }
        if let Some(watts) = node.current_watts {
            let w = watts.abs();
            let kw = w / 1000.0;
            let value_str = if kw >= 1.0 {
                format!("{:.1} kW", kw)
            } else {
                format!("{:.0} W", w)
            };
            let desc = match (node.id, &node.direction) {
                ("solar", _) => format!("solar producing {}", value_str),
                ("grid", FlowDirection::ToHouse) => format!("importing {} from grid", value_str),
                ("grid", FlowDirection::FromHouse) => format!("exporting {} to grid", value_str),
                ("grid", FlowDirection::Idle) => "grid idle".to_string(),
                ("battery", FlowDirection::ToHouse) => format!("battery discharging {}", value_str),
                ("battery", FlowDirection::FromHouse) => format!("battery charging {}", value_str),
                ("battery", FlowDirection::Idle) => "battery idle".to_string(),
                ("ev", _) => format!("EV charging {}", value_str),
                _ => continue,
            };
            parts.push(desc);
        }
    }

    if parts.is_empty() {
        "Energy flow: no active sources".to_string()
    } else {
        format!("Energy flow: {}", parts.join(", "))
    }
}

/// Derive status announcement text from the current energy state.
/// Returns None if called with current state that matches previous announcement.
pub fn derive_status_announcement(
    nodes: &[SchematicNodeState],
    stale: bool,
    previous: &str,
) -> Option<String> {
    // MQTT disconnection is highest-priority announcement
    if stale {
        let text = "Connection lost — showing last known values".to_string();
        if text == previous {
            return None;
        }
        return Some(text);
    }

    // Derive current state description for key status changes
    let mut announcements = Vec::new();

    for node in nodes {
        match node.id {
            "grid" if node.visible => {
                match node.direction {
                    FlowDirection::ToHouse => announcements.push("importing from grid"),
                    FlowDirection::FromHouse => announcements.push("exporting to grid"),
                    _ => {}
                }
            }
            "battery" if node.visible => {
                match node.direction {
                    FlowDirection::ToHouse => announcements.push("battery discharging"),
                    FlowDirection::FromHouse => announcements.push("battery charging"),
                    _ => {}
                }
            }
            "ev" if node.visible => {
                if node.current_watts.map_or(false, |w| w.abs() > 50.0) {
                    announcements.push("EV charging");
                }
            }
            "solar" if !node.visible => {
                announcements.push("solar production stopped");
            }
            _ => {}
        }
    }

    let text = if announcements.is_empty() {
        "System idle".to_string()
    } else {
        announcements.join(", ").to_string()
    };

    // Only announce if changed
    if text == previous {
        None
    } else {
        Some(text)
    }
}

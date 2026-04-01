use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type SharedState = Arc<RwLock<CurrentState>>;

#[derive(Debug, Clone, Default, Serialize)]
pub struct CurrentState {
    pub site: SiteState,
    pub loadpoints: HashMap<u32, LoadpointState>,
    pub last_updated: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SiteState {
    pub grid_power: Option<f64>,
    pub grid_energy: Option<f64>,
    pub grid_currents: [Option<f64>; 3],
    pub pv_power: Option<f64>,
    pub pv_energy: Option<f64>,
    pub home_power: Option<f64>,
    pub battery_power: Option<f64>,
    pub battery_soc: Option<f64>,
    pub battery_energy: Option<f64>,
    pub battery_capacity: Option<f64>,
    pub green_share_home: Option<f64>,
    pub self_sufficiency_pct: Option<f64>,
}

impl SiteState {
    pub fn grid_power_display(&self) -> String {
        self.grid_power.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn pv_power_display(&self) -> String {
        self.pv_power.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn home_power_display(&self) -> String {
        self.home_power.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn battery_power_display(&self) -> String {
        self.battery_power.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn battery_soc_display(&self) -> String {
        self.battery_soc.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn grid_css_class(&self) -> &str {
        match self.grid_power {
            Some(v) if v < 0.0 => "export",
            _ => "import",
        }
    }
    pub fn battery_css_class(&self) -> &str {
        // evcc: positive = discharging, negative = charging
        match self.battery_power {
            Some(v) if v < 0.0 => "charging",
            Some(v) if v > 0.0 => "discharging",
            _ => "",
        }
    }
    pub fn battery_status_text(&self) -> &str {
        // evcc: positive = discharging, negative = charging
        match self.battery_power {
            Some(v) if v < 0.0 => "(charging)",
            Some(v) if v > 0.0 => "(discharging)",
            Some(_) => "(idle)",
            None => "",
        }
    }
    pub fn grid_direction(&self) -> String {
        match self.grid_power {
            Some(v) if v > 0.0 => format!("{:.0} W import", v),
            Some(v) if v < 0.0 => format!("{:.0} W export", v.abs()),
            Some(_) => "0 W".to_string(),
            None => "--".to_string(),
        }
    }
    pub fn grid_direction_css(&self) -> &str {
        match self.grid_power {
            Some(v) if v > 0.0 => "import",
            Some(v) if v < 0.0 => "export",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct LoadpointState {
    pub id: u32,
    pub title: Option<String>,
    pub charge_power: Option<f64>,
    pub charged_energy: Option<f64>,
    pub charge_total_import: Option<f64>,
    pub charging: Option<bool>,
    pub connected: Option<bool>,
    pub enabled: Option<bool>,
    pub vehicle_name: Option<String>,
    pub vehicle_soc: Option<f64>,
    pub vehicle_range: Option<f64>,
    pub mode: Option<String>,
}

impl LoadpointState {
    pub fn charge_power_display(&self) -> String {
        self.charge_power.map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn title_display(&self) -> String {
        self.title.clone().unwrap_or_else(|| format!("Loadpoint {}", self.id))
    }
    pub fn is_connected(&self) -> bool {
        self.connected.unwrap_or(false)
    }
    pub fn vehicle_soc_display(&self) -> String {
        self.vehicle_soc
            .filter(|v| *v > 0.0)
            .map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn vehicle_range_display(&self) -> String {
        self.vehicle_range
            .filter(|v| *v > 0.0)
            .map_or("--".to_string(), |v| format!("{:.0}", v))
    }
    pub fn charged_energy_display(&self) -> String {
        // evcc sends chargedEnergy in Wh
        self.charged_energy.map_or("-- kWh".to_string(), |wh| {
            let kwh = wh / 1000.0;
            if kwh >= 1000.0 {
                format!("{:.2} MWh", kwh / 1000.0)
            } else {
                format!("{:.1} kWh", kwh)
            }
        })
    }
    pub fn mode_display(&self) -> String {
        self.mode.clone().unwrap_or_else(|| "--".to_string())
    }
    pub fn vehicle_name_display(&self) -> String {
        self.vehicle_name.clone().unwrap_or_default()
    }
    pub fn has_vehicle(&self) -> bool {
        self.vehicle_name.is_some()
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct EnergyTotals {
    pub pv_production_wh: Option<f64>,
    pub grid_import_wh: Option<f64>,
    pub grid_export_wh: Option<f64>,
    pub home_consumption_wh: Option<f64>,
    pub battery_charge_wh: Option<f64>,
    pub battery_discharge_wh: Option<f64>,
    pub sample_count: i64,
}

impl EnergyTotals {
    fn format_wh(wh: Option<f64>) -> String {
        match wh {
            Some(v) if v >= 1000.0 => format!("{:.2} kWh", v / 1000.0),
            Some(v) => format!("{:.0} Wh", v),
            None => "--".to_string(),
        }
    }
    pub fn pv_display(&self) -> String { Self::format_wh(self.pv_production_wh) }
    pub fn grid_import_display(&self) -> String { Self::format_wh(self.grid_import_wh) }
    pub fn grid_export_display(&self) -> String { Self::format_wh(self.grid_export_wh) }
    pub fn home_display(&self) -> String { Self::format_wh(self.home_consumption_wh) }
    pub fn battery_charge_display(&self) -> String { Self::format_wh(self.battery_charge_wh) }
    pub fn battery_discharge_display(&self) -> String { Self::format_wh(self.battery_discharge_wh) }
    pub fn self_sufficiency_display(&self) -> String {
        match (self.home_consumption_wh, self.grid_import_wh) {
            (Some(home), Some(grid)) if home > 0.0 => {
                format!("{:.1}%", 100.0 * (1.0 - grid / home))
            }
            _ => "--".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EnergySample {
    pub timestamp: i64,
    pub grid_power: Option<f64>,
    pub pv_power: Option<f64>,
    pub home_power: Option<f64>,
    pub battery_power: Option<f64>,
    pub battery_soc: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadpointSample {
    pub timestamp: i64,
    pub loadpoint_id: u32,
    pub charge_power: Option<f64>,
    pub charged_energy: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChartData {
    pub timestamps: Vec<i64>,
    pub series: HashMap<String, Vec<Option<f64>>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct DailySummary {
    pub date: String,
    pub grid_import_wh: Option<f64>,
    pub grid_export_wh: Option<f64>,
    pub pv_production_wh: Option<f64>,
    pub home_consumption_wh: Option<f64>,
    pub battery_charge_wh: Option<f64>,
    pub battery_discharge_wh: Option<f64>,
    pub self_sufficiency_pct: Option<f64>,
}

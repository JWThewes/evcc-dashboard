use crate::model::CurrentState;

pub fn parse_message(prefix: &str, topic: &str, _payload: &[u8]) -> bool {
    topic.starts_with(prefix)
}

pub fn apply_message(state: &mut CurrentState, prefix: &str, topic: &str, payload: &[u8]) {
    let payload_str = match std::str::from_utf8(payload) {
        Ok(s) => s.trim(),
        Err(_) => return,
    };

    let suffix = match topic.strip_prefix(prefix) {
        Some(s) => s.trim_start_matches('/'),
        None => return,
    };

    match suffix {
        // Nested path format (actual evcc topics)
        "site/grid/power" => {
            state.site.grid_power = payload_str.parse().ok();
        }
        "site/grid/energy" => {
            state.site.grid_energy = payload_str.parse().ok();
        }
        "site/grid/currents/1" => {
            state.site.grid_currents[0] = payload_str.parse().ok();
        }
        "site/grid/currents/2" => {
            state.site.grid_currents[1] = payload_str.parse().ok();
        }
        "site/grid/currents/3" => {
            state.site.grid_currents[2] = payload_str.parse().ok();
        }
        "site/battery/power" => {
            state.site.battery_power = payload_str.parse().ok();
        }
        "site/battery/soc" => {
            state.site.battery_soc = payload_str.parse().ok();
        }
        "site/battery/energy" => {
            state.site.battery_energy = payload_str.parse().ok();
        }
        "site/battery/capacity" => {
            state.site.battery_capacity = payload_str.parse().ok();
        }

        // CamelCase format (also published by evcc)
        "site/gridPower" | "site/grid/Power" => {
            state.site.grid_power = payload_str.parse().ok();
        }
        "site/pvPower" => {
            state.site.pv_power = payload_str.parse().ok();
        }
        "site/pvEnergy" => {
            state.site.pv_energy = payload_str.parse().ok();
        }
        "site/homePower" => {
            state.site.home_power = payload_str.parse().ok();
        }
        "site/batteryPower" => {
            state.site.battery_power = payload_str.parse().ok();
        }
        "site/batterySoc" => {
            state.site.battery_soc = payload_str.parse().ok();
        }

        // Self-consumption and statistics
        "site/greenShareHome" => {
            state.site.green_share_home = payload_str.parse().ok();
        }
        "site/statistics/30d/solarPercentage" => {
            state.site.self_sufficiency_pct = payload_str.parse().ok();
        }

        other => {
            if let Some(rest) = other.strip_prefix("loadpoints/") {
                parse_loadpoint_message(state, rest, payload_str);
            } else if other.starts_with("site/") {
                tracing::trace!("Unhandled site topic: {other} = {payload_str}");
            }
        }
    }

    state.last_updated = Some(chrono::Utc::now().timestamp());
}

fn parse_loadpoint_message(state: &mut CurrentState, path: &str, value: &str) {
    let parts: Vec<&str> = path.splitn(2, '/').collect();
    if parts.len() != 2 {
        return;
    }

    let id: u32 = match parts[0].parse() {
        Ok(id) => id,
        Err(_) => return,
    };

    let lp = state.loadpoints.entry(id).or_default();
    lp.id = id;

    match parts[1] {
        "chargePower" => {
            lp.charge_power = value.parse().ok();
        }
        "chargedEnergy" => {
            // Session energy in Wh
            lp.charged_energy = value.parse().ok();
        }
        "chargeTotalImport" => {
            // Total meter reading in Wh
            lp.charge_total_import = value.parse().ok();
        }
        "charging" => {
            lp.charging = value.parse().ok();
        }
        "vehicleName" => {
            lp.vehicle_name = Some(value.to_string());
        }
        "mode" => {
            lp.mode = Some(value.to_string());
        }
        "title" => {
            lp.title = Some(value.to_string());
        }
        "vehicleTitle" => {
            lp.vehicle_name = Some(value.to_string());
        }
        "vehicleSoc" => {
            lp.vehicle_soc = value.parse().ok();
        }
        "vehicleRange" => {
            lp.vehicle_range = value.parse().ok();
        }
        "connected" => {
            lp.connected = value.parse().ok();
        }
        "enabled" => {
            lp.enabled = value.parse().ok();
        }
        other => {
            tracing::trace!("Unhandled loadpoint topic: loadpoints/{id}/{other} = {value}");
        }
    }
}

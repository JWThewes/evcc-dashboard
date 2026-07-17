//! Energy Flow unit integration tests.
//!
//! Validates:
//! - EnergyFlowState construction from SiteState
//! - Self-sufficiency calculation with noise floor
//! - Flow path direction logic (forward/reverse/idle)
//! - Animation duration inverse-linear calculation
//! - OOB swap target IDs (contract-oob-swap-ids: 14 IDs)
//! - Asset budget compliance (CSS < 4KB, JS < 2KB compressed)
//! - No external URLs in CSS/JS assets
//! - Reduced-motion rule presence in CSS
//! - No eval/innerHTML in JS
//! - Security: no inline event handlers in templates

use evcc_dashboard::model::SiteState;
use evcc_dashboard::web::routes::energy_flow::{EnergyFlowState, FlowDirection};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// EnergyFlowState Construction Tests
// ---------------------------------------------------------------------------

#[test]
fn energy_flow_state_from_default_site() {
    let site = SiteState::default();
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.self_sufficiency, 0.0);
    assert!(!state.pv_path.active);
    assert!(!state.grid_path.active);
    assert!(!state.battery_path.active);
    assert!(!state.ev_path.active);
}

#[test]
fn energy_flow_state_pv_active_above_noise_floor() {
    let mut site = SiteState::default();
    site.pv_power = Some(500.0);
    site.home_power = Some(500.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.pv_path.active);
    assert_eq!(state.pv_path.direction, FlowDirection::Forward);
}

#[test]
fn energy_flow_state_pv_idle_below_noise_floor() {
    let mut site = SiteState::default();
    site.pv_power = Some(5.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(!state.pv_path.active);
    assert_eq!(state.pv_path.direction, FlowDirection::Idle);
}

#[test]
fn energy_flow_state_grid_import_is_forward() {
    let mut site = SiteState::default();
    site.grid_power = Some(1000.0); // positive = import
    site.home_power = Some(1000.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.grid_path.active);
    assert_eq!(state.grid_path.direction, FlowDirection::Forward);
}

#[test]
fn energy_flow_state_grid_export_is_reverse() {
    let mut site = SiteState::default();
    site.grid_power = Some(-2000.0); // negative = export
    site.home_power = Some(500.0);
    site.pv_power = Some(2500.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.grid_path.active);
    assert_eq!(state.grid_path.direction, FlowDirection::Reverse);
    assert!(state.grid_path.is_reverse());
}

#[test]
fn energy_flow_state_battery_discharge_is_forward() {
    let mut site = SiteState::default();
    site.battery_power = Some(800.0); // positive = discharging
    site.battery_soc = Some(60.0);
    site.home_power = Some(800.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.has_battery);
    assert!(state.battery_path.active);
    assert_eq!(state.battery_path.direction, FlowDirection::Forward);
}

#[test]
fn energy_flow_state_battery_charging_is_reverse() {
    let mut site = SiteState::default();
    site.battery_power = Some(-1500.0); // negative = charging
    site.battery_soc = Some(30.0);
    site.home_power = Some(500.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.has_battery);
    assert!(state.battery_path.active);
    assert_eq!(state.battery_path.direction, FlowDirection::Reverse);
}

#[test]
fn energy_flow_state_ev_active_when_connected() {
    let mut site = SiteState::default();
    site.ev_power = Some(3500.0);
    site.ev_connected = true;
    site.home_power = Some(3500.0);
    let state = EnergyFlowState::from_site(&site);
    assert!(state.has_ev);
    assert!(state.ev_path.active);
    assert_eq!(state.ev_path.direction, FlowDirection::Forward);
}

// ---------------------------------------------------------------------------
// Self-Sufficiency Calculation Tests
// ---------------------------------------------------------------------------

#[test]
fn self_sufficiency_100_percent_when_no_grid_import() {
    let mut site = SiteState::default();
    site.pv_power = Some(3000.0);
    site.home_power = Some(2500.0);
    site.grid_power = Some(-500.0); // exporting
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.self_sufficiency, 100.0);
}

#[test]
fn self_sufficiency_0_when_all_from_grid() {
    let mut site = SiteState::default();
    site.home_power = Some(2000.0);
    site.grid_power = Some(2000.0); // all imported
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.self_sufficiency, 0.0);
}

#[test]
fn self_sufficiency_50_percent_half_from_grid() {
    let mut site = SiteState::default();
    site.home_power = Some(2000.0);
    site.grid_power = Some(1000.0); // half imported
    site.pv_power = Some(1000.0);
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.self_sufficiency, 50.0);
}

#[test]
fn self_sufficiency_clamped_to_0_100() {
    let mut site = SiteState::default();
    site.home_power = Some(5.0); // below noise floor
    site.grid_power = Some(100.0);
    let state = EnergyFlowState::from_site(&site);
    // home < noise floor → 0%
    assert_eq!(state.self_sufficiency, 0.0);
}

// ---------------------------------------------------------------------------
// Animation Duration Tests
// ---------------------------------------------------------------------------

#[test]
fn duration_min_at_max_power() {
    let mut site = SiteState::default();
    site.pv_power = Some(10000.0);
    site.home_power = Some(10000.0);
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.pv_path.duration_ms, 1500);
}

#[test]
fn duration_max_at_noise_floor() {
    let mut site = SiteState::default();
    site.pv_power = Some(10.0); // at noise floor
    site.home_power = Some(10.0);
    let state = EnergyFlowState::from_site(&site);
    assert_eq!(state.pv_path.duration_ms, 6000);
}

#[test]
fn duration_scales_linearly() {
    let mut site = SiteState::default();
    site.pv_power = Some(5000.0); // roughly midpoint
    site.home_power = Some(5000.0);
    let state = EnergyFlowState::from_site(&site);
    // Should be roughly midpoint between 1500 and 6000
    assert!(state.pv_path.duration_ms > 2000);
    assert!(state.pv_path.duration_ms < 5000);
}

// ---------------------------------------------------------------------------
// Asset Compliance Tests
// ---------------------------------------------------------------------------

#[test]
fn energy_flow_css_exists_and_within_budget() {
    let path = Path::new("static/css/energy-flow.css");
    assert!(path.exists(), "energy-flow.css must exist");
    let content = fs::read_to_string(path).unwrap();
    // Uncompressed budget check (4KB compressed ≈ 8KB uncompressed for CSS)
    assert!(
        content.len() < 8192,
        "energy-flow.css exceeds 8KB uncompressed: {} bytes",
        content.len()
    );
}

#[test]
fn energy_flow_js_exists_and_within_budget() {
    let path = Path::new("static/js/energy-flow.js");
    assert!(path.exists(), "energy-flow.js must exist");
    let content = fs::read_to_string(path).unwrap();
    // Uncompressed budget check (2KB compressed ≈ 4KB uncompressed for JS)
    assert!(
        content.len() < 4096,
        "energy-flow.js exceeds 4KB uncompressed: {} bytes",
        content.len()
    );
}

#[test]
fn energy_flow_css_no_external_urls() {
    let content = fs::read_to_string("static/css/energy-flow.css").unwrap();
    assert!(
        !content.contains("://"),
        "energy-flow.css must not contain external URLs"
    );
}

#[test]
fn energy_flow_js_no_external_urls() {
    let content = fs::read_to_string("static/js/energy-flow.js").unwrap();
    assert!(
        !content.contains("://"),
        "energy-flow.js must not contain external URLs"
    );
}

#[test]
fn energy_flow_js_no_eval_or_innerhtml() {
    let content = fs::read_to_string("static/js/energy-flow.js").unwrap();
    assert!(
        !content.contains("eval("),
        "energy-flow.js must not use eval()"
    );
    assert!(
        !content.contains("innerHTML"),
        "energy-flow.js must not use innerHTML"
    );
    assert!(
        !content.contains("new Function"),
        "energy-flow.js must not use new Function"
    );
}

#[test]
fn energy_flow_css_has_reduced_motion_rule() {
    let content = fs::read_to_string("static/css/energy-flow.css").unwrap();
    assert!(
        content.contains("prefers-reduced-motion"),
        "energy-flow.css must include prefers-reduced-motion media query"
    );
}

#[test]
fn energy_flow_css_uses_ef_prefix() {
    let content = fs::read_to_string("static/css/energy-flow.css").unwrap();
    // All class selectors should use .ef- prefix
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('.') && trimmed.contains('{') {
            // Class selector line — should start with .ef-
            let class_part = trimmed.split('{').next().unwrap().trim();
            for selector in class_part.split(',') {
                let sel = selector.trim();
                if sel.starts_with('.') {
                    assert!(
                        sel.starts_with(".ef-"),
                        "CSS class selector '{sel}' must use .ef- prefix"
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OOB Swap Contract Tests
// ---------------------------------------------------------------------------

#[test]
fn oob_template_contains_all_14_contract_ids() {
    let content = fs::read_to_string("templates/partials/energy_flow_oob.html").unwrap();
    let required_ids = [
        "ef-pv-power",
        "ef-grid-power",
        "ef-home-power",
        "ef-battery-power",
        "ef-ev-power",
        "ef-self-sufficiency",
        "ef-pv-path-power",
        "ef-grid-path-power",
        "ef-battery-path-power",
        "ef-ev-path-power",
        "ef-pv-path-dir",
        "ef-grid-path-dir",
        "ef-battery-path-dir",
        "ef-ev-path-dir",
    ];
    for id in &required_ids {
        assert!(
            content.contains(&format!("id=\"{id}\"")),
            "OOB template missing contract ID: {id}"
        );
    }
}

#[test]
fn oob_template_all_spans_have_hx_swap_oob() {
    let content = fs::read_to_string("templates/partials/energy_flow_oob.html").unwrap();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<span") && trimmed.contains("id=\"ef-") {
            assert!(
                trimmed.contains("hx-swap-oob=\"true\""),
                "OOB span missing hx-swap-oob: {trimmed}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Security Tests
// ---------------------------------------------------------------------------

#[test]
fn svg_template_no_inline_event_handlers() {
    let content =
        fs::read_to_string("templates/partials/energy_flow_svg.html").unwrap();
    let dangerous_attrs = [
        "onclick", "onload", "onerror", "onmouseover", "onfocus",
    ];
    for attr in &dangerous_attrs {
        assert!(
            !content.to_lowercase().contains(attr),
            "SVG template must not contain inline event handler: {attr}"
        );
    }
}

#[test]
fn svg_template_no_script_tags() {
    let content =
        fs::read_to_string("templates/partials/energy_flow_svg.html").unwrap();
    assert!(
        !content.to_lowercase().contains("<script"),
        "SVG template must not contain <script> tags"
    );
}

#[test]
fn svg_template_no_external_references() {
    let content =
        fs::read_to_string("templates/partials/energy_flow_svg.html").unwrap();
    // Check for external URLs (but allow xmlns declarations)
    for line in content.lines() {
        if line.contains("://") && !line.contains("xmlns") {
            panic!(
                "SVG template must not reference external URLs: {}",
                line.trim()
            );
        }
    }
}

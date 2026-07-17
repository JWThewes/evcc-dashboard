//! ECharts Integration unit tests.
//!
//! Validates:
//! - charts.js existence and structure
//! - Theme engine contract compliance (THEME_NAME = "evcc-energy")
//! - CSS token consumption (15 tokens read by getComputedTokens)
//! - Fallback color completeness
//! - No eval/innerHTML/dynamic code execution patterns
//! - No external network requests
//! - Script loading order in shell template
//! - Asset-containment compatibility (allowlist compliance)
//! - data-testid attributes for automation
//! - Series label completeness

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// File Existence Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_exists() {
    assert!(
        Path::new("static/js/charts.js").exists(),
        "static/js/charts.js must exist"
    );
}

#[test]
fn echarts_vendor_exists() {
    assert!(
        Path::new("static/js/echarts.min.js").exists(),
        "static/js/echarts.min.js must exist (vendored ECharts library)"
    );
}

// ---------------------------------------------------------------------------
// Theme Contract Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_registers_evcc_energy_theme() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains(r#"THEME_NAME = "evcc-energy""#),
        "charts.js must define THEME_NAME as 'evcc-energy' (contract-echarts-theme-name)"
    );
}

#[test]
fn charts_js_calls_register_theme() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("echarts.registerTheme(THEME_NAME"),
        "charts.js must call echarts.registerTheme with THEME_NAME"
    );
}

#[test]
fn charts_js_theme_registration_before_chart_init() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    // registerEnergyTheme must appear before initChart calls in DOMContentLoaded
    let reg_pos = content.find("registerEnergyTheme()");
    let init_pos = content.find("initChart");
    assert!(
        reg_pos.is_some() && init_pos.is_some(),
        "Both registerEnergyTheme and initChart must exist"
    );
    // Within DOMContentLoaded, registration should come first
    // Find the DOMContentLoaded handler
    let dcl_pos = content.find("DOMContentLoaded").unwrap();
    let content_after_dcl = &content[dcl_pos..];
    let reg_in_dcl = content_after_dcl.find("registerEnergyTheme");
    let init_in_dcl = content_after_dcl.find("initChart");
    assert!(
        reg_in_dcl.unwrap() < init_in_dcl.unwrap(),
        "registerEnergyTheme must execute before initChart (Rule 1: registration precedes initialization)"
    );
}

// ---------------------------------------------------------------------------
// CSS Token Consumption Tests (15 tokens per contract-css-tokens)
// ---------------------------------------------------------------------------

#[test]
fn charts_js_reads_all_fifteen_css_tokens() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    let required_tokens = [
        "--color-solar",
        "--color-grid",
        "--color-battery",
        "--color-ev",
        "--color-home",
        "--color-surface-0",
        "--color-surface-1",
        "--color-surface-2",
        "--color-surface-3",
        "--color-text-primary",
        "--color-text-secondary",
        "--color-text-muted",
        "--color-positive",
        "--color-negative",
        "--color-neutral",
    ];
    for token in &required_tokens {
        assert!(
            content.contains(token),
            "charts.js must reference CSS token '{token}'"
        );
    }
}

#[test]
fn charts_js_uses_get_computed_style() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("getComputedStyle"),
        "charts.js must use getComputedStyle to read live CSS token values"
    );
}

// ---------------------------------------------------------------------------
// Fallback Color Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_has_fallback_colors_for_all_tokens() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("FALLBACK_COLORS"),
        "charts.js must define FALLBACK_COLORS map"
    );
    // Verify all 15 token fallbacks are present
    let required_fallbacks = [
        "--color-solar",
        "--color-grid",
        "--color-battery",
        "--color-ev",
        "--color-home",
        "--color-surface-0",
        "--color-surface-1",
        "--color-surface-2",
        "--color-surface-3",
        "--color-text-primary",
        "--color-text-secondary",
        "--color-text-muted",
        "--color-positive",
        "--color-negative",
        "--color-neutral",
    ];
    // Find the FALLBACK_COLORS block
    let fb_start = content.find("FALLBACK_COLORS").expect("FALLBACK_COLORS must exist");
    // Look for the closing brace (approximate — find next `};`)
    let fb_section = &content[fb_start..fb_start + 1500.min(content.len() - fb_start)];
    for token in &required_fallbacks {
        assert!(
            fb_section.contains(token),
            "FALLBACK_COLORS must include fallback for '{token}'"
        );
    }
}

// ---------------------------------------------------------------------------
// Security Tests: No Dynamic Code Execution
// ---------------------------------------------------------------------------

#[test]
fn charts_js_no_eval() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    // Check for eval( but not "evaluation" or similar words
    let has_eval = content.contains("eval(") || content.contains("eval (");
    assert!(
        !has_eval,
        "charts.js must not use eval() (CSP compliance)"
    );
}

#[test]
fn charts_js_no_inner_html() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        !content.contains("innerHTML"),
        "charts.js must not use innerHTML (XSS prevention)"
    );
}

#[test]
fn charts_js_no_new_function() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        !content.contains("new Function("),
        "charts.js must not use new Function() (CSP compliance)"
    );
}

#[test]
fn charts_js_no_dynamic_import() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        !content.contains("import("),
        "charts.js must not use dynamic import() (network isolation)"
    );
}

// ---------------------------------------------------------------------------
// Network Isolation Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_no_external_urls() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    // Should not contain any http:// or https:// external URLs
    for line in content.lines() {
        // Skip comment lines that might reference specs/docs
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("*") {
            continue;
        }
        assert!(
            !line.contains("http://") && !line.contains("https://"),
            "charts.js must not contain external URLs in code. Found in: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// Script Loading Order Tests
// ---------------------------------------------------------------------------

#[test]
fn shell_template_loads_echarts_before_charts_js() {
    let content = fs::read_to_string("templates/shell.html").unwrap();
    let echarts_pos = content.find("echarts.min.js");
    let charts_pos = content.find("charts.js");
    assert!(
        echarts_pos.is_some(),
        "shell.html must load echarts.min.js"
    );
    assert!(
        charts_pos.is_some(),
        "shell.html must load charts.js"
    );
    assert!(
        echarts_pos.unwrap() < charts_pos.unwrap(),
        "echarts.min.js must be loaded before charts.js (dependency order)"
    );
}

#[test]
fn shell_template_loads_charts_js_with_defer() {
    let content = fs::read_to_string("templates/shell.html").unwrap();
    // Find the script tag for charts.js and verify it has defer
    let charts_line = content
        .lines()
        .find(|l| l.contains("charts.js"))
        .expect("shell.html must reference charts.js");
    assert!(
        charts_line.contains("defer"),
        "charts.js script tag must have defer attribute"
    );
}

#[test]
fn shell_template_loads_echarts_with_defer() {
    let content = fs::read_to_string("templates/shell.html").unwrap();
    let echarts_line = content
        .lines()
        .find(|l| l.contains("echarts.min.js"))
        .expect("shell.html must reference echarts.min.js");
    assert!(
        echarts_line.contains("defer"),
        "echarts.min.js script tag must have defer attribute"
    );
}

// ---------------------------------------------------------------------------
// Theme Toggle Integration Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_listens_for_theme_changed_event() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("theme-changed"),
        "charts.js must listen for 'theme-changed' custom event"
    );
}

#[test]
fn charts_js_has_idempotent_listener_guard() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("_themeListenerRegistered"),
        "charts.js must use a listener guard to prevent duplicate registrations"
    );
}

// ---------------------------------------------------------------------------
// Chart Lifecycle Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_has_init_chart_function() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("function initChart"),
        "charts.js must define initChart function"
    );
}

#[test]
fn charts_js_has_chart_registry() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("var charts = {}") || content.contains("var charts={}"),
        "charts.js must define a charts registry object"
    );
}

#[test]
fn charts_js_disposes_before_reinit() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    // The rerenderCharts function should dispose instances before re-init
    let rerender_pos = content.find("function rerenderCharts").unwrap();
    let rerender_section = &content[rerender_pos..];
    let dispose_pos = rerender_section.find("dispose()");
    let init_pos = rerender_section.find("echarts.init(");
    assert!(
        dispose_pos.is_some() && init_pos.is_some(),
        "rerenderCharts must call dispose and init"
    );
    assert!(
        dispose_pos.unwrap() < init_pos.unwrap(),
        "dispose() must be called before echarts.init() in rerenderCharts (Rule 7)"
    );
}

#[test]
fn charts_js_checks_is_connected() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("isConnected"),
        "charts.js must check element.isConnected for registry cleanup (Rule 8)"
    );
}

// ---------------------------------------------------------------------------
// Auto-Refresh Tests
// ---------------------------------------------------------------------------

#[test]
fn charts_js_has_auto_refresh() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("startChartRefresh"),
        "charts.js must define startChartRefresh for 30s auto-refresh"
    );
    assert!(
        content.contains("30000") || content.contains("30 * 1000"),
        "Auto-refresh interval must be 30 seconds (30000ms)"
    );
}

// ---------------------------------------------------------------------------
// Color Palette Order Tests (Rule 9: energy semantics)
// ---------------------------------------------------------------------------

#[test]
fn charts_js_palette_follows_energy_semantic_order() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    // Find the color array in buildThemeObject
    let build_pos = content
        .find("function buildThemeObject")
        .expect("buildThemeObject must exist");
    let build_section = &content[build_pos..build_pos + 2000.min(content.len() - build_pos)];

    // The color array should follow: solar, grid, battery, ev, home, positive, negative
    let color_array_start = build_section.find("color: [").expect("color array must exist");
    let color_section = &build_section[color_array_start..];
    let color_end = color_section.find(']').unwrap();
    let color_str = &color_section[..color_end];

    // Verify order by finding positions of token references
    let solar_pos = color_str.find("solar").expect("solar must be in palette");
    let grid_pos = color_str.find("grid").expect("grid must be in palette");
    let battery_pos = color_str.find("battery").expect("battery must be in palette");
    let ev_pos = color_str.find("ev").expect("ev must be in palette");
    let home_pos = color_str.find("home").expect("home must be in palette");
    let positive_pos = color_str.find("positive").expect("positive must be in palette");
    let negative_pos = color_str.find("negative").expect("negative must be in palette");

    assert!(solar_pos < grid_pos, "Solar must be before grid in palette");
    assert!(grid_pos < battery_pos, "Grid must be before battery");
    assert!(battery_pos < ev_pos, "Battery must be before EV");
    assert!(ev_pos < home_pos, "EV must be before home");
    assert!(home_pos < positive_pos, "Home must be before positive");
    assert!(positive_pos < negative_pos, "Positive must be before negative");
}

// ---------------------------------------------------------------------------
// Transparent Background Test (Rule 3)
// ---------------------------------------------------------------------------

#[test]
fn charts_js_uses_transparent_background() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains(r#"backgroundColor: "transparent""#),
        "ECharts theme must use transparent background (Rule 3)"
    );
}

// ---------------------------------------------------------------------------
// Graceful Degradation Tests (Rule 5, ECharts unavailability)
// ---------------------------------------------------------------------------

#[test]
fn charts_js_guards_against_missing_echarts() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains(r#"typeof echarts === "undefined""#)
            || content.contains(r#"typeof echarts==="undefined""#),
        "charts.js must guard against missing echarts global (graceful degradation)"
    );
}

#[test]
fn charts_js_logs_warning_when_echarts_missing() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("console.warn"),
        "charts.js must use console.warn for degradation logging"
    );
}

// ---------------------------------------------------------------------------
// Series Labels Test
// ---------------------------------------------------------------------------

#[test]
fn charts_js_has_series_labels() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(content.contains("LABELS"), "charts.js must define LABELS map");
    // Verify key series are labeled
    let required_labels = [
        "grid_power",
        "pv_power",
        "home_power",
        "battery_power",
        "battery_soc",
    ];
    for label in &required_labels {
        assert!(
            content.contains(label),
            "LABELS must include key '{label}'"
        );
    }
}

// ---------------------------------------------------------------------------
// Range Selector Test
// ---------------------------------------------------------------------------

#[test]
fn charts_js_has_update_chart_range() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("function updateChartRange"),
        "charts.js must define updateChartRange for range selector interaction"
    );
}

// ---------------------------------------------------------------------------
// Window Resize Handling Test
// ---------------------------------------------------------------------------

#[test]
fn charts_js_handles_window_resize() {
    let content = fs::read_to_string("static/js/charts.js").unwrap();
    assert!(
        content.contains("resize"),
        "charts.js must handle window resize events"
    );
}

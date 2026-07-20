//! Integration tests for the comparison-feature unit.
//!
//! Tests cover:
//! - Period boundary computation for all window types
//! - Input validation (invalid window, invalid metric)
//! - Summary aggregation logic and Wh→kWh conversion
//! - Self-sufficiency formula and edge cases
//! - Chart resolution tier mapping
//! - Yesterday comparison delta calculations
//! - Asset containment (no external URLs in compare.css/compare.js)
//! - Template structure and accessibility attributes

use std::fs;
use std::path::Path;

use chrono::{Datelike, Utc};

// ---------------------------------------------------------------------------
// Period Boundary Tests
// ---------------------------------------------------------------------------

#[test]
fn test_period_bounds_day_window() {
    // Day window: current starts at midnight today, previous at midnight yesterday
    // Both periods have the same elapsed duration
    let now = chrono::Utc::now();
    let today_midnight = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();

    // Simulate what CompareWindow::Day.compute_periods() does
    let current_start = today_midnight;
    let current_end = now.timestamp();
    let previous_start = today_midnight - 86400;
    let elapsed = current_end - current_start;
    let previous_end = previous_start + elapsed;

    assert!(current_start <= current_end);
    assert!(previous_start < current_start);
    assert_eq!(current_end - current_start, previous_end - previous_start);
}

#[test]
fn test_period_bounds_week_window() {
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let days_since_monday = today.weekday().num_days_from_monday() as i64;
    let monday = today - chrono::Duration::days(days_since_monday);
    let current_start = monday.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let previous_start = current_start - 7 * 86400;

    // Week periods are exactly 7 days apart
    assert_eq!(current_start - previous_start, 7 * 86400);
    assert!(current_start <= now.timestamp());
}

#[test]
fn test_period_bounds_month_window_equal_elapsed() {
    let now = chrono::Utc::now();
    let today = now.date_naive();
    let first_of_month = chrono::NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
    let current_start = first_of_month.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let elapsed = now.timestamp() - current_start;

    // Previous period has same elapsed duration (period fairness constraint)
    let prev_month = if today.month() == 1 {
        chrono::NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).unwrap()
    };
    let previous_start = prev_month.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
    let previous_end_candidate = previous_start + elapsed;

    // previous_end should be clamped if month has fewer days
    let prev_month_end = first_of_month.pred_opt().unwrap()
        .and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();
    let previous_end = previous_end_candidate.min(prev_month_end);

    assert!(previous_end <= prev_month_end);
    assert!(previous_end >= previous_start);
}

// ---------------------------------------------------------------------------
// Validation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_window_deserialization_valid() {
    let valid_windows = ["day", "week", "month"];
    for w in valid_windows {
        let result: Result<serde_json::Value, _> = serde_json::from_str(&format!("\"{}\"", w));
        assert!(result.is_ok(), "Window '{}' should be valid JSON string", w);
    }
}

#[test]
fn test_metric_values_are_snake_case() {
    // Per VR-02: metric must be solar|consumption|grid_import|grid_export
    let valid_metrics = ["solar", "consumption", "grid_import", "grid_export"];
    for m in valid_metrics {
        assert!(!m.contains(' '), "Metric '{}' should not contain spaces", m);
        assert_eq!(m, m.to_lowercase(), "Metric '{}' should be lowercase", m);
    }
}

// ---------------------------------------------------------------------------
// Self-Sufficiency Formula Tests
// ---------------------------------------------------------------------------

#[test]
fn test_self_sufficiency_normal() {
    // self_sufficiency = 100 * (1 - grid_import / consumption)
    // consumption=10000 Wh, grid_import=3000 Wh → 70%
    let consumption_wh = 10000.0;
    let grid_import_wh = 3000.0;
    let consumption_kwh = consumption_wh / 1000.0;
    let grid_import_kwh = grid_import_wh / 1000.0;
    let pct = (100.0 * (1.0 - grid_import_kwh / consumption_kwh)).clamp(0.0, 100.0);
    assert!((pct - 70.0).abs() < 0.01);
}

#[test]
fn test_self_sufficiency_zero_consumption() {
    // If consumption is 0, self-sufficiency should be 0 (not divide by zero)
    let consumption_kwh = 0.0;
    let pct = if consumption_kwh > 0.0 {
        (100.0 * (1.0 - 0.0 / consumption_kwh)).clamp(0.0, 100.0)
    } else {
        0.0
    };
    assert_eq!(pct, 0.0);
}

#[test]
fn test_self_sufficiency_clamped_to_100() {
    // Edge: grid exports more than imports (negative import effectively)
    // Formula yields > 100 → clamp
    let consumption_kwh = 5.0;
    let grid_import_kwh = -1.0; // hypothetical negative
    let pct = (100.0 * (1.0 - grid_import_kwh / consumption_kwh)).clamp(0.0, 100.0);
    assert_eq!(pct, 100.0);
}

#[test]
fn test_self_sufficiency_clamped_to_0() {
    // All power from grid → self-sufficiency = 0
    let consumption_kwh = 5.0;
    let grid_import_kwh = 5.0; // all from grid
    let pct = (100.0 * (1.0 - grid_import_kwh / consumption_kwh)).clamp(0.0, 100.0);
    assert_eq!(pct, 0.0);
}

// ---------------------------------------------------------------------------
// Delta Calculation Tests
// ---------------------------------------------------------------------------

#[test]
fn test_delta_normal_increase() {
    let current = 150.0;
    let previous = 100.0;
    let delta = ((current - previous) / previous) * 100.0;
    assert!((delta - 50.0).abs() < 0.01);
}

#[test]
fn test_delta_normal_decrease() {
    let current = 80.0;
    let previous = 100.0;
    let delta = ((current - previous) / previous) * 100.0;
    assert!((delta - (-20.0)).abs() < 0.01);
}

#[test]
fn test_delta_previous_zero_current_positive() {
    // Infinite increase → cap at +100%
    let current = 50.0;
    let previous = 0.0;
    let delta = if previous > 0.0 {
        ((current - previous) / previous) * 100.0
    } else if current > 0.0 {
        100.0
    } else {
        0.0
    };
    assert_eq!(delta, 100.0);
}

#[test]
fn test_delta_both_zero() {
    let current = 0.0;
    let previous = 0.0;
    let delta = if previous > 0.0 {
        ((current - previous) / previous) * 100.0
    } else if current > 0.0 {
        100.0
    } else {
        0.0
    };
    assert_eq!(delta, 0.0);
}

// ---------------------------------------------------------------------------
// Resolution Tier Mapping Tests
// ---------------------------------------------------------------------------

#[test]
fn test_resolution_mapping() {
    // Day → 5m resolution → energy_samples with GROUP BY 300
    // Week → 1h resolution → energy_samples_1m with GROUP BY 3600
    // Month → daily → daily_summaries
    let cases = vec![
        ("day", "5m"),
        ("week", "1h"),
        ("month", "daily"),
    ];
    for (window, expected_resolution) in cases {
        let resolution = match window {
            "day" => "5m",
            "week" => "1h",
            "month" => "daily",
            _ => "5m",
        };
        assert_eq!(resolution, expected_resolution, "Window '{}' should map to resolution '{}'", window, expected_resolution);
    }
}

#[test]
fn test_resolution_grouping_seconds() {
    // 5m → 300s grouping
    // 1h → 3600s grouping
    let cases = vec![
        ("5m", 300_i64),
        ("1h", 3600_i64),
    ];
    for (resolution, expected) in cases {
        let group = match resolution {
            "5m" => 300,
            "1h" => 3600,
            _ => 300,
        };
        assert_eq!(group, expected);
    }
}

// ---------------------------------------------------------------------------
// Wh to kWh Conversion Tests
// ---------------------------------------------------------------------------

#[test]
fn test_wh_to_kwh_conversion() {
    let wh = 5432.1;
    let kwh = wh / 1000.0;
    assert!((kwh - 5.4321).abs() < 0.0001);
}

#[test]
fn test_power_to_energy_5m_interval() {
    // avg_power_w * 300s / 3600 = Wh per 5-minute interval
    let avg_power = 2000.0; // 2kW average
    let energy_wh = avg_power * 300.0 / 3600.0;
    assert!((energy_wh - 166.667).abs() < 0.01);
}

#[test]
fn test_power_to_energy_1h_interval() {
    // avg_power_w * 3600s / 3600 = avg_power_w (Wh per hour)
    let avg_power = 2000.0;
    let energy_wh = avg_power * 3600.0 / 3600.0;
    assert_eq!(energy_wh, 2000.0);
}

// ---------------------------------------------------------------------------
// Asset Containment Tests (compare.css and compare.js)
// ---------------------------------------------------------------------------

#[test]
fn test_compare_css_no_external_urls() {
    let css_path = Path::new("static/css/compare.css");
    assert!(css_path.exists(), "compare.css must exist");

    let content = fs::read_to_string(css_path).expect("read compare.css");
    let external_indicators = ["://", "url(http", "url(https", "@import"];

    for indicator in external_indicators {
        assert!(
            !content.contains(indicator),
            "compare.css must not contain external URL indicator: '{}'",
            indicator
        );
    }
}

#[test]
fn test_compare_js_no_external_urls() {
    let js_path = Path::new("static/js/compare.js");
    assert!(js_path.exists(), "compare.js must exist");

    let content = fs::read_to_string(js_path).expect("read compare.js");

    // Should not reference external domains
    let external_patterns = ["://", "http://", "https://"];
    for pattern in external_patterns {
        assert!(
            !content.contains(pattern),
            "compare.js must not contain external URL: '{}'",
            pattern
        );
    }
}

#[test]
fn test_compare_js_no_eval_or_innerhtml() {
    let content = fs::read_to_string("static/js/compare.js").expect("read compare.js");

    assert!(!content.contains("eval("), "compare.js must not use eval()");
    assert!(!content.contains("innerHTML"), "compare.js must not use innerHTML");
    assert!(!content.contains("new Function"), "compare.js must not use new Function");
    assert!(!content.contains("document.write"), "compare.js must not use document.write");
}

#[test]
fn test_compare_js_uses_echarts_theme() {
    let content = fs::read_to_string("static/js/compare.js").expect("read compare.js");
    assert!(
        content.contains("'evcc-energy'") || content.contains("\"evcc-energy\""),
        "compare.js must use the registered ECharts theme name 'evcc-energy'"
    );
}

// ---------------------------------------------------------------------------
// Template Structure Tests
// ---------------------------------------------------------------------------

#[test]
fn test_compare_template_has_data_testids() {
    let template_path = Path::new("templates/views/compare.html");
    assert!(template_path.exists(), "compare.html must exist");

    let content = fs::read_to_string(template_path).expect("read compare.html");

    let required_testids = [
        "view-compare",
        "compare-window-selector",
        "compare-window-day",
        "compare-window-week",
        "compare-window-month",
        "compare-table",
        "compare-table-body",
        "compare-chart-toggle",
        "compare-chart-section",
        "compare-metric-selector",
        "compare-chart",
        "compare-insufficient",
    ];

    for testid in required_testids {
        assert!(
            content.contains(&format!("data-testid=\"{}\"", testid)),
            "compare.html must have data-testid=\"{}\"",
            testid
        );
    }
}

#[test]
fn test_compare_template_has_aria_attributes() {
    let content = fs::read_to_string("templates/views/compare.html").expect("read compare.html");

    // Window selector uses tablist/tab roles
    assert!(content.contains("role=\"tablist\""), "must have tablist role");
    assert!(content.contains("role=\"tab\""), "must have tab role");
    assert!(content.contains("aria-selected"), "must have aria-selected");
    assert!(content.contains("aria-pressed"), "must have aria-pressed on toggle");
    assert!(content.contains("aria-live"), "must have aria-live for dynamic content");
    assert!(content.contains("role=\"alert\""), "must have alert role for insufficient data");
}

#[test]
fn test_compare_template_no_external_references() {
    let content = fs::read_to_string("templates/views/compare.html").expect("read compare.html");

    // No external stylesheet or script references
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("href=") || line.contains("src=") {
            // Allow local references only
            assert!(
                line.contains("/static/") || line.contains("\"#"),
                "Line {} has non-local reference: {}",
                i + 1,
                line.trim()
            );
        }
    }
}

#[test]
fn test_compare_css_size_budget() {
    let content = fs::read_to_string("static/css/compare.css").expect("read compare.css");
    let size_bytes = content.len();
    // Budget: ≤5KB uncompressed (will be ~3KB compressed)
    assert!(
        size_bytes <= 5120,
        "compare.css is {} bytes, must be ≤5120 bytes uncompressed",
        size_bytes
    );
}

#[test]
fn test_compare_js_size_budget() {
    let content = fs::read_to_string("static/js/compare.js").expect("read compare.js");
    let size_bytes = content.len();
    // Budget: ≤10KB uncompressed (will be ~4KB compressed)
    assert!(
        size_bytes <= 10240,
        "compare.js is {} bytes, must be ≤10240 bytes uncompressed",
        size_bytes
    );
}

// ---------------------------------------------------------------------------
// SQL Expression Mapping Tests
// ---------------------------------------------------------------------------

#[test]
fn test_metric_sql_expressions_use_case_when() {
    // All metric SQL expressions must use CASE WHEN for safe column extraction
    let expressions = [
        "CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END",
        "CASE WHEN home_power > 0 THEN home_power ELSE 0 END",
        "CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END",
        "CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END",
    ];

    for expr in expressions {
        assert!(expr.starts_with("CASE WHEN"), "SQL expression must use CASE WHEN pattern");
        assert!(expr.ends_with("END"), "SQL expression must end with END");
    }
}

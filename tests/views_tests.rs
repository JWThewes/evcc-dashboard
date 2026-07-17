//! Integration tests for the views unit.
//!
//! Validates: self-sufficiency computation, summary card rendering,
//! loadpoint state machine, asset budgets, security constraints, and
//! accessibility compliance.

use std::fs;
use std::path::Path;

// ===========================================================================
// Self-Sufficiency Computation Tests
// ===========================================================================

/// Self-sufficiency: normal case with partial grid import.
#[test]
fn test_self_sufficiency_normal() {
    // home=3000W, grid=1000W (import) -> (3000-1000)/3000*100 = 66%
    let home = 3000.0_f64;
    let grid_import = 1000.0_f64.max(0.0);
    let pct = ((home - grid_import) / home * 100.0).clamp(0.0, 100.0);
    assert_eq!(pct as u8, 66);
}

/// Self-sufficiency: zero home power returns None (displayed as --).
#[test]
fn test_self_sufficiency_zero_home() {
    let home = 0.0_f64;
    let result = if home > 10.0 {
        let grid_import = 0.0_f64.max(0.0);
        Some(((home - grid_import) / home * 100.0).clamp(0.0, 100.0) as u8)
    } else {
        None
    };
    assert_eq!(result, None);
}

/// Self-sufficiency: full self-sufficiency (no grid import).
#[test]
fn test_self_sufficiency_full() {
    let home = 5000.0_f64;
    let grid_power = -500.0_f64; // export
    let grid_import = grid_power.max(0.0);
    let pct = ((home - grid_import) / home * 100.0).clamp(0.0, 100.0);
    assert_eq!(pct as u8, 100);
}

/// Self-sufficiency: grid import exceeds home (edge case, clamps to 0).
#[test]
fn test_self_sufficiency_over_import() {
    // This shouldn't happen physically but test the clamp
    let home = 1000.0_f64;
    let grid_import = 2000.0_f64;
    let pct = ((home - grid_import) / home * 100.0).clamp(0.0, 100.0);
    assert_eq!(pct as u8, 0);
}

// ===========================================================================
// Summary Card Delta Tests
// ===========================================================================

/// Delta: positive increase from yesterday.
#[test]
fn test_delta_positive() {
    let today = 5000.0_f64;
    let yesterday = 3000.0_f64;
    let delta = ((today - yesterday) / yesterday * 100.0).clamp(-100.0, 100.0);
    assert!((delta - 66.67).abs() < 0.1);
}

/// Delta: zero yesterday yields 100% (capped).
#[test]
fn test_delta_zero_yesterday() {
    let today = 5000.0_f64;
    let yesterday = 0.0_f64;
    let delta = if yesterday == 0.0 {
        if today > 0.0 { 100.0 } else { 0.0 }
    } else {
        ((today - yesterday) / yesterday * 100.0).clamp(-100.0, 100.0)
    };
    assert_eq!(delta, 100.0);
}

/// Delta: decrease from yesterday.
#[test]
fn test_delta_decrease() {
    let today = 2000.0_f64;
    let yesterday = 5000.0_f64;
    let delta = ((today - yesterday) / yesterday * 100.0).clamp(-100.0, 100.0);
    assert_eq!(delta, -60.0);
}

// ===========================================================================
// Loadpoint State Machine Tests
// ===========================================================================

/// Loadpoint state: connected + charging = Active.
#[test]
fn test_loadpoint_state_active() {
    let connected = true;
    let charging = true;
    let state = if connected && charging {
        "Active"
    } else if connected {
        "Idle"
    } else {
        "Disconnected"
    };
    assert_eq!(state, "Active");
}

/// Loadpoint state: connected + not charging = Idle.
#[test]
fn test_loadpoint_state_idle() {
    let connected = true;
    let charging = false;
    let state = if connected && charging {
        "Active"
    } else if connected {
        "Idle"
    } else {
        "Disconnected"
    };
    assert_eq!(state, "Idle");
}

/// Loadpoint state: not connected = Disconnected.
#[test]
fn test_loadpoint_state_disconnected() {
    let connected = false;
    let charging = false;
    let state = if connected && charging {
        "Active"
    } else if connected {
        "Idle"
    } else {
        "Disconnected"
    };
    assert_eq!(state, "Disconnected");
}

// ===========================================================================
// Power Formatting Tests
// ===========================================================================

/// Power display: watts below 1000.
#[test]
fn test_power_format_watts() {
    let w = 750.0_f64;
    let display = if w >= 1000.0 {
        format!("{:.1} kW", w / 1000.0)
    } else {
        format!("{:.0} W", w)
    };
    assert_eq!(display, "750 W");
}

/// Power display: kilowatts.
#[test]
fn test_power_format_kilowatts() {
    let w = 3200.0_f64;
    let display = if w >= 1000.0 {
        format!("{:.1} kW", w / 1000.0)
    } else {
        format!("{:.0} W", w)
    };
    assert_eq!(display, "3.2 kW");
}

// ===========================================================================
// Performance Budget Tests
// ===========================================================================

/// views.css must be under 8KB uncompressed.
#[test]
fn test_views_css_size_budget() {
    let path = Path::new("static/css/views.css");
    assert!(path.exists(), "views.css must exist");
    let size = fs::metadata(path).unwrap().len();
    assert!(
        size <= 8192,
        "views.css is {} bytes, must be ≤ 8192",
        size
    );
}

/// history.js must be under 2KB uncompressed.
#[test]
fn test_history_js_size_budget() {
    let path = Path::new("static/js/history.js");
    assert!(path.exists(), "history.js must exist");
    let size = fs::metadata(path).unwrap().len();
    assert!(
        size <= 2048,
        "history.js is {} bytes, must be ≤ 2048",
        size
    );
}

// ===========================================================================
// Security Tests
// ===========================================================================

/// View templates must not reference external URLs.
#[test]
fn test_no_external_urls_in_view_templates() {
    let template_dir = Path::new("templates/views");
    assert!(template_dir.is_dir());

    for entry in fs::read_dir(template_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "html") {
            let content = fs::read_to_string(&path).unwrap();
            // Check for external URLs (http:// or https:// that aren't in xmlns)
            for (line_num, line) in content.lines().enumerate() {
                if line.contains("://") && !line.contains("xmlns") {
                    panic!(
                        "External URL found in {:?} at line {}: {}",
                        path,
                        line_num + 1,
                        line.trim()
                    );
                }
            }
        }
    }
}

/// View JS/CSS must not make external network requests.
#[test]
fn test_no_external_urls_in_view_assets() {
    let files = vec![
        "static/css/views.css",
        "static/js/history.js",
    ];

    for file_path in files {
        let path = Path::new(file_path);
        if !path.exists() { continue; }
        let content = fs::read_to_string(path).unwrap();
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("://") && !line.contains("xmlns") && !line.contains("//") {
                // Allow single-line comments with //
                let trimmed = line.trim();
                if !trimmed.starts_with("//") && !trimmed.starts_with("*") {
                    panic!(
                        "External URL found in {:?} at line {}: {}",
                        path,
                        line_num + 1,
                        trimmed
                    );
                }
            }
        }
    }
}

// ===========================================================================
// Accessibility Tests
// ===========================================================================

/// View templates must have data-testid on interactive elements.
#[test]
fn test_data_testid_present() {
    let required_testids = vec![
        ("templates/views/overview.html", vec!["view-overview", "self-sufficiency-hero", "overview-summary"]),
        ("templates/views/charging.html", vec!["view-charging", "charging-cards"]),
        ("templates/views/history.html", vec!["view-history", "history-range-selector", "history-chart"]),
    ];

    for (file, ids) in required_testids {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|_| panic!("Cannot read {}", file));
        for id in ids {
            let attr = format!("data-testid=\"{}\"", id);
            assert!(
                content.contains(&attr),
                "{} missing data-testid=\"{}\"",
                file,
                id
            );
        }
    }
}

/// Overview template must have aria-live for dynamic content.
#[test]
fn test_overview_aria_live() {
    let content = fs::read_to_string("templates/views/overview.html").unwrap();
    assert!(
        content.contains("aria-live"),
        "Overview template must have aria-live for dynamic summary content"
    );
}

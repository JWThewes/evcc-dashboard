//! Foundation unit integration tests.
//!
//! Validates:
//! - Static asset existence and budget compliance
//! - No external requests (no Google Fonts references)
//! - Required ARIA attributes in templates
//! - ViewId correctness and uniqueness
//! - Theme cookie parsing and defaults
//! - Design token contract completeness

use evcc_dashboard::web::routes::views::{Theme, ViewId};
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// ViewId Tests
// ---------------------------------------------------------------------------

#[test]
fn viewid_all_returns_five_views() {
    assert_eq!(ViewId::all().len(), 5);
}

#[test]
fn viewid_paths_are_unique() {
    let paths: Vec<&str> = ViewId::all().iter().map(|v| v.path()).collect();
    let mut deduped = paths.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(paths.len(), deduped.len(), "ViewId paths must be unique");
}

#[test]
fn viewid_from_path_matches_all_views() {
    assert_eq!(ViewId::from_path("/overview"), ViewId::Overview);
    assert_eq!(ViewId::from_path("/charging"), ViewId::Charging);
    assert_eq!(ViewId::from_path("/history"), ViewId::History);
    assert_eq!(ViewId::from_path("/compare"), ViewId::Compare);
    assert_eq!(ViewId::from_path("/settings"), ViewId::Settings);
}

#[test]
fn viewid_from_path_defaults_to_overview() {
    assert_eq!(ViewId::from_path("/unknown"), ViewId::Overview);
    assert_eq!(ViewId::from_path("/"), ViewId::Overview);
    assert_eq!(ViewId::from_path(""), ViewId::Overview);
}

// ---------------------------------------------------------------------------
// Theme Tests
// ---------------------------------------------------------------------------

#[test]
fn theme_defaults_to_dark() {
    assert_eq!(Theme::default(), Theme::Dark);
}

#[test]
fn theme_from_cookie_parses_light() {
    assert_eq!(Theme::from_cookie("light"), Theme::Light);
    assert_eq!(Theme::from_cookie(" light "), Theme::Light);
}

#[test]
fn theme_from_cookie_defaults_to_dark_for_unknown() {
    assert_eq!(Theme::from_cookie("invalid"), Theme::Dark);
    assert_eq!(Theme::from_cookie(""), Theme::Dark);
    assert_eq!(Theme::from_cookie("DARK"), Theme::Dark);
}

#[test]
fn theme_as_str_roundtrips() {
    assert_eq!(Theme::Dark.as_str(), "dark");
    assert_eq!(Theme::Light.as_str(), "light");
}

// ---------------------------------------------------------------------------
// Static Asset Tests
// ---------------------------------------------------------------------------

#[test]
fn tokens_css_exists() {
    assert!(
        Path::new("static/css/tokens.css").exists(),
        "tokens.css must exist"
    );
}

#[test]
fn shell_css_exists() {
    assert!(
        Path::new("static/css/shell.css").exists(),
        "shell.css must exist"
    );
}

#[test]
fn theme_js_exists() {
    assert!(
        Path::new("static/js/theme.js").exists(),
        "theme.js must exist"
    );
}

#[test]
fn asset_budget_individual_files_under_4096_bytes() {
    let files = [
        "static/css/tokens.css",
        "static/css/shell.css",
        "static/js/theme.js",
    ];
    for path in &files {
        let size = fs::metadata(path)
            .unwrap_or_else(|_| panic!("{path} must exist"))
            .len();
        assert!(
            size <= 4096,
            "{path} is {size} bytes, exceeds 4096 byte budget"
        );
    }
}

#[test]
fn asset_budget_total_under_16384_bytes() {
    let files = [
        "static/css/tokens.css",
        "static/css/shell.css",
        "static/js/theme.js",
    ];
    let total: u64 = files
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
        .sum();
    assert!(
        total <= 16384,
        "Total new assets are {total} bytes, exceeds 16384 byte budget"
    );
}

// ---------------------------------------------------------------------------
// No External Requests Tests
// ---------------------------------------------------------------------------

#[test]
fn no_google_fonts_in_templates() {
    let template_dir = Path::new("templates");
    check_no_external_refs(template_dir);
}

fn check_no_external_refs(dir: &Path) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            check_no_external_refs(&path);
        } else if path.extension().is_some_and(|e| e == "html") {
            let content = fs::read_to_string(&path).unwrap();
            assert!(
                !content.contains("fonts.googleapis.com"),
                "Template {} references Google Fonts",
                path.display()
            );
            assert!(
                !content.contains("fonts.gstatic.com"),
                "Template {} references Google Fonts static",
                path.display()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Template Structure Tests
// ---------------------------------------------------------------------------

#[test]
fn shell_template_has_required_aria() {
    let content = fs::read_to_string("templates/shell.html").unwrap();
    assert!(content.contains(r#"role="tablist""#), "Missing tablist role");
    assert!(content.contains(r#"role="tab""#), "Missing tab role");
    assert!(content.contains("aria-selected"), "Missing aria-selected");
    assert!(content.contains("aria-label"), "Missing aria-label");
    assert!(content.contains(r#"role="main""#), "Missing main role");
    assert!(content.contains("skip-link"), "Missing skip-to-content link");
}

#[test]
fn tokens_css_has_dark_and_light_themes() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(
        content.contains(":root"),
        "tokens.css must define :root (dark default)"
    );
    assert!(
        content.contains("[data-theme=\"light\"]"),
        "tokens.css must define [data-theme=\"light\"] override"
    );
}

#[test]
fn tokens_css_has_reduced_motion() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(
        content.contains("prefers-reduced-motion"),
        "tokens.css must include prefers-reduced-motion media query"
    );
}

#[test]
fn tokens_css_has_energy_color_tokens() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(content.contains("--color-solar"), "Missing --color-solar");
    assert!(content.contains("--color-grid"), "Missing --color-grid");
    assert!(content.contains("--color-battery"), "Missing --color-battery");
    assert!(content.contains("--color-ev"), "Missing --color-ev");
    assert!(content.contains("--color-home"), "Missing --color-home");
}

#[test]
fn tokens_css_has_surface_tokens() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(content.contains("--color-surface-0"), "Missing --color-surface-0");
    assert!(content.contains("--color-surface-1"), "Missing --color-surface-1");
    assert!(content.contains("--color-surface-2"), "Missing --color-surface-2");
    assert!(content.contains("--color-surface-3"), "Missing --color-surface-3");
}

#[test]
fn tokens_css_has_spacing_tokens() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(content.contains("--space-1"), "Missing --space-1");
    assert!(content.contains("--space-4"), "Missing --space-4");
    assert!(content.contains("--space-8"), "Missing --space-8");
}

#[test]
fn tokens_css_has_typography_tokens() {
    let content = fs::read_to_string("static/css/tokens.css").unwrap();
    assert!(content.contains("--font-family"), "Missing --font-family");
    assert!(content.contains("--font-size-base"), "Missing --font-size-base");
}

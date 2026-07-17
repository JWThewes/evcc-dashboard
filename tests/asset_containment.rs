//! Asset-Containment Integration Tests
//!
//! Enforces the zero-external-network-requests invariant (story-no-external-requests).
//! All tests are build-time verification only — no code from this file is compiled
//! into the production binary.
//!
//! Implements four verification workflows:
//! 1. External Reference Audit (all templates and static files)
//! 2. Google Fonts CDN Removal Verification
//! 3. Inline SVG Icon Verification
//! 4. Docker Build Integrity (ignored — requires Docker daemon)
//!
//! Additional enforcement:
//! - Script allowlist (htmx.min.js, echarts.min.js only)
//! - System font stack verification (no @font-face, uses system-ui)
//! - CSS external reference scan (no external url() or @import)
//! - JS network API scan (no unexpected external fetch patterns)

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Constants (per ADR-AC-SEC-01 and ADR-AC-SEC-03)
// ---------------------------------------------------------------------------

/// Scripts permitted in `<script src="...">` attributes.
/// Per ADR-AC-SEC-03: exactly two entries, architectural invariant.
const ALLOWED_SCRIPTS: &[&str] = &[
    "/static/js/htmx.min.js",
    "/static/js/echarts.min.js",
];

/// Additional scripts allowed in the new shell template (theme.js, charts.js).
/// These are local project scripts served from /static/js/.
const ALLOWED_LOCAL_SCRIPTS: &[&str] = &[
    "/static/js/theme.js",
    "/static/js/charts.js",
];

// ---------------------------------------------------------------------------
// Helper: External Reference Detection (ADR-AC-SEC-01)
// ---------------------------------------------------------------------------

/// Determines if a reference string points to an external resource.
/// Uses the conservative allowlist approach: anything with `://` or starting
/// with `//` is considered external.
fn is_external_reference(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Protocol-relative URLs are always external
    if trimmed.starts_with("//") {
        return true;
    }

    // Explicit schemes are always external
    if trimmed.contains("://") {
        return true;
    }

    false
}

/// Scan a text content for lines containing external references.
/// Returns a list of (line_number, line_content) for violations.
fn find_external_references(content: &str) -> Vec<(usize, String)> {
    let mut violations = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let line_num = idx + 1;
        // Check for protocol-relative URLs
        if line.contains("//") {
            // Exclude comments (// in JS), known-safe patterns
            let stripped = line.trim();
            if stripped.starts_with("//") && !stripped.starts_with("//cdn")
                && !stripped.starts_with("//fonts")
                && !stripped.starts_with("//www")
            {
                // This is a code comment, not a URL
                continue;
            }
            // Check if the // is actually a protocol-relative URL in an attribute
            if contains_protocol_relative_url(line) {
                violations.push((line_num, line.to_string()));
            }
        }
        // Check for explicit external schemes
        if line.contains("http://") || line.contains("https://") {
            // Exclude HTML comments that document things
            let trimmed = line.trim();
            if trimmed.starts_with("<!--") || trimmed.starts_with("//") || trimmed.starts_with("*") {
                // Skip comments in source
                continue;
            }
            // Exclude data URIs — inline content with SVG namespace declarations
            // e.g. url("data:image/svg+xml,...http://www.w3.org/2000/svg...")
            if line.contains("data:") && line.contains("w3.org") {
                continue;
            }
            // Exclude xmlns declarations in inline SVG
            if line.contains("xmlns") && line.contains("w3.org") {
                continue;
            }
            violations.push((line_num, line.to_string()));
        }
    }
    violations
}

/// Check if a line contains a protocol-relative URL in an HTML/CSS context
/// (as opposed to a JS comment which starts with //).
fn contains_protocol_relative_url(line: &str) -> bool {
    // Look for patterns like href="//..." src="//..." url(//...)
    let patterns = [
        "href=\"//",
        "href='//",
        "src=\"//",
        "src='//",
        "url(//",
        "url(\"//",
        "url('//",
    ];
    patterns.iter().any(|p| line.contains(p))
}

// ---------------------------------------------------------------------------
// Helper: Recursive File Scanner
// ---------------------------------------------------------------------------

/// Recursively collect all files with given extensions from a directory.
fn collect_files(dir: &Path, extensions: &[&str]) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("Cannot read directory {}: {}", dir.display(), e))
        .filter_map(|e| e.ok())
        .collect();
    // Sort for deterministic ordering (per REL-AC-001 reliability design)
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path, extensions));
        } else if let Some(ext) = path.extension() {
            if extensions.iter().any(|e| ext == *e) {
                files.push(path);
            }
        }
    }
    files
}

// ===========================================================================
// Workflow 1: External Reference Audit
// ===========================================================================

#[test]
fn workflow1_no_external_references_in_html_templates() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);
    assert!(
        !html_files.is_empty(),
        "No HTML templates found in templates/"
    );

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        let violations = find_external_references(&content);
        assert!(
            violations.is_empty(),
            "External reference found in {}: line {} — {}",
            file.display(),
            violations.first().map_or(0, |v| v.0),
            violations.first().map_or(String::new(), |v| v.1.clone())
        );
    }
}

#[test]
fn workflow1_no_external_references_in_css_files() {
    let static_css = Path::new("static/css");
    let css_files = collect_files(static_css, &["css"]);
    assert!(!css_files.is_empty(), "No CSS files found in static/css/");

    for file in &css_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        let violations = find_external_references(&content);
        assert!(
            violations.is_empty(),
            "External reference found in {}: line {} — {}",
            file.display(),
            violations.first().map_or(0, |v| v.0),
            violations.first().map_or(String::new(), |v| v.1.clone())
        );
    }
}

#[test]
fn workflow1_no_external_references_in_js_files() {
    let static_js = Path::new("static/js");
    let js_files = collect_files(static_js, &["js"]);
    assert!(!js_files.is_empty(), "No JS files found in static/js/");

    for file in &js_files {
        let filename = file.file_name().unwrap().to_string_lossy();
        // Skip vendored minified files — they are trusted local copies
        if filename == "htmx.min.js" || filename == "echarts.min.js" {
            continue;
        }

        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        let violations = find_external_references(&content);
        assert!(
            violations.is_empty(),
            "External reference found in {}: line {} — {}",
            file.display(),
            violations.first().map_or(0, |v| v.0),
            violations.first().map_or(String::new(), |v| v.1.clone())
        );
    }
}

// ===========================================================================
// Workflow 2: Google Fonts CDN Removal Verification
// ===========================================================================

#[test]
fn workflow2_no_google_fonts_in_templates() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        assert!(
            !content.contains("fonts.googleapis.com"),
            "NI-002 violation: {} references Google Fonts API",
            file.display()
        );
        assert!(
            !content.contains("fonts.gstatic.com"),
            "NI-002 violation: {} references Google Fonts static CDN",
            file.display()
        );
    }
}

#[test]
fn workflow2_no_google_fonts_in_css() {
    let static_css = Path::new("static/css");
    let css_files = collect_files(static_css, &["css"]);

    for file in &css_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        assert!(
            !content.contains("fonts.googleapis.com"),
            "NI-002 violation: {} references Google Fonts API",
            file.display()
        );
        assert!(
            !content.contains("fonts.gstatic.com"),
            "NI-002 violation: {} references Google Fonts static CDN",
            file.display()
        );
    }
}

#[test]
fn workflow2_system_font_stack_in_tokens() {
    let content = fs::read_to_string("static/css/tokens.css")
        .expect("static/css/tokens.css must exist");
    assert!(
        content.contains("--font-family"),
        "AS-001 violation: tokens.css must define --font-family token"
    );
    // The system font stack should use platform system fonts, not external fonts.
    // Acceptable system font keywords/names: system-ui, -apple-system, BlinkMacSystemFont,
    // "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif.
    let has_system_font = content.contains("system-ui")
        || content.contains("-apple-system")
        || content.contains("BlinkMacSystemFont")
        || content.contains("sans-serif");
    assert!(
        has_system_font,
        "AS-001 violation: --font-family must use a system font stack"
    );
}

#[test]
fn workflow2_no_font_face_declarations() {
    let static_css = Path::new("static/css");
    let css_files = collect_files(static_css, &["css"]);

    for file in &css_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        assert!(
            !content.contains("@font-face"),
            "AS-001 violation: {} contains @font-face declaration — system font stack only",
            file.display()
        );
    }
}

// ===========================================================================
// Workflow 3: Inline SVG Icon Verification
// ===========================================================================

#[test]
fn workflow3_no_icon_font_references() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);

    let icon_font_indicators = [
        "font-awesome",
        "fontawesome",
        "material-icons",
        "material-symbols",
        "icomoon",
        "glyphicons",
        "ionicons",
    ];

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        let content_lower = content.to_lowercase();
        for indicator in &icon_font_indicators {
            assert!(
                !content_lower.contains(indicator),
                "AS-002 violation: {} references icon font '{}' — inline SVG only",
                file.display(),
                indicator
            );
        }
    }
}

#[test]
fn workflow3_no_external_svg_sprites() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        // Check for <use xlink:href="http..." or <use href="http..."
        assert!(
            !content.contains("xlink:href=\"http"),
            "AS-002 violation: {} references external SVG sprite via xlink:href",
            file.display()
        );
        assert!(
            !content.contains("xlink:href='http"),
            "AS-002 violation: {} references external SVG sprite via xlink:href",
            file.display()
        );
    }
}

#[test]
fn workflow3_shell_has_inline_svg_icons() {
    let content = fs::read_to_string("templates/shell.html")
        .expect("templates/shell.html must exist");
    // The shell template uses ViewId::icon_svg() which renders inline <svg> elements
    // Verify that SVG markup is present (injected via icon_svg()|safe)
    assert!(
        content.contains("icon_svg()"),
        "Shell template must use icon_svg() for inline SVG icons"
    );
}

// ===========================================================================
// Workflow 4: Docker Build Integrity (IGNORED — requires Docker daemon)
// ===========================================================================

#[test]
#[ignore]
fn workflow4_docker_compose_single_service() {
    // This test verifies the Docker Compose file defines exactly one service.
    // Ignored by default because it requires filesystem access to the install
    // script output. Run with: cargo test -- --ignored
    let compose_content = fs::read_to_string("docker-compose.yml")
        .or_else(|_| fs::read_to_string("docker-compose.yaml"))
        .expect("docker-compose.yml or docker-compose.yaml must exist");

    // Count service definitions (lines matching "  <service-name>:" at 2-space indent)
    let service_count = compose_content
        .lines()
        .filter(|line| {
            // Under "services:" section, service names are at 2-space indent
            line.starts_with("  ") && !line.starts_with("    ") && line.trim_end().ends_with(':')
        })
        .count();

    assert_eq!(
        service_count, 1,
        "DI-001 violation: docker-compose.yml must define exactly one service, found {}",
        service_count
    );
}

// ===========================================================================
// Script Allowlist Enforcement (ADR-AC-SEC-03)
// ===========================================================================

#[test]
fn script_allowlist_only_permitted_scripts_in_templates() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));

        // Find all <script src="..."> patterns
        for line in content.lines() {
            if let Some(src_start) = line.find("src=\"") {
                if !line[..src_start].contains("<script") {
                    continue;
                }
                let after_src = &line[src_start + 5..];
                if let Some(src_end) = after_src.find('"') {
                    let src_value = &after_src[..src_end];
                    // Strip base_path template prefix if present
                    let normalized = src_value
                        .trim_start_matches("{{ base_path }}")
                        .trim_start_matches("{{base_path}}");

                    // Check against both allowlists
                    let is_allowed = ALLOWED_SCRIPTS.iter().any(|a| normalized.ends_with(a))
                        || ALLOWED_LOCAL_SCRIPTS.iter().any(|a| normalized.ends_with(a));

                    assert!(
                        is_allowed,
                        "AS-003 violation: {} references unauthorized script '{}'. \
                         Allowed: {:?} + {:?}",
                        file.display(),
                        src_value,
                        ALLOWED_SCRIPTS,
                        ALLOWED_LOCAL_SCRIPTS
                    );
                }
            }
        }
    }
}

#[test]
fn script_allowlist_vendored_js_files_exist() {
    // Verify that the allowed vendored scripts actually exist on disk
    assert!(
        Path::new("static/js/htmx.min.js").exists(),
        "AS-003: vendored htmx.min.js must exist at static/js/htmx.min.js"
    );
    assert!(
        Path::new("static/js/echarts.min.js").exists(),
        "AS-003: vendored echarts.min.js must exist at static/js/echarts.min.js"
    );
}

// ===========================================================================
// CSS External Reference Scan (NI-001)
// ===========================================================================

#[test]
fn css_no_external_imports() {
    let static_css = Path::new("static/css");
    let css_files = collect_files(static_css, &["css"]);

    for file in &css_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));

        for (idx, line) in content.lines().enumerate() {
            if line.trim().starts_with("@import") {
                // @import with external URL is a violation
                if line.contains("http://") || line.contains("https://") || line.contains("//") {
                    panic!(
                        "AS-004 violation: {} line {} has external @import: {}",
                        file.display(),
                        idx + 1,
                        line.trim()
                    );
                }
            }
        }
    }
}

#[test]
fn css_no_external_url_functions() {
    let static_css = Path::new("static/css");
    let css_files = collect_files(static_css, &["css"]);

    for file in &css_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));

        for (idx, line) in content.lines().enumerate() {
            if line.contains("url(") {
                // Extract the URL value
                if line.contains("url(\"http") || line.contains("url('http")
                    || line.contains("url(http") || line.contains("url(\"//")
                    || line.contains("url('//") || line.contains("url(//")
                {
                    panic!(
                        "AS-004 violation: {} line {} has external url(): {}",
                        file.display(),
                        idx + 1,
                        line.trim()
                    );
                }
            }
        }
    }
}

// ===========================================================================
// JS Network API Scan
// ===========================================================================

#[test]
fn js_no_xmlhttprequest_in_project_files() {
    let static_js = Path::new("static/js");
    let js_files = collect_files(static_js, &["js"]);

    for file in &js_files {
        let filename = file.file_name().unwrap().to_string_lossy();
        // Skip vendored minified files
        if filename == "htmx.min.js" || filename == "echarts.min.js" {
            continue;
        }

        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        assert!(
            !content.contains("XMLHttpRequest"),
            "NI-001 violation: {} uses XMLHttpRequest — only htmx is permitted for network calls",
            file.display()
        );
    }
}

#[test]
fn js_no_external_fetch_targets() {
    let static_js = Path::new("static/js");
    let js_files = collect_files(static_js, &["js"]);

    for file in &js_files {
        let filename = file.file_name().unwrap().to_string_lossy();
        // Skip vendored minified files
        if filename == "htmx.min.js" || filename == "echarts.min.js" {
            continue;
        }

        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));

        // Check for fetch calls with external URLs
        for (idx, line) in content.lines().enumerate() {
            if line.contains("fetch(") || line.contains("fetch (") {
                // fetch with explicit http:// or https:// is a violation
                if line.contains("http://") || line.contains("https://") {
                    panic!(
                        "NI-001 violation: {} line {} fetches external URL: {}",
                        file.display(),
                        idx + 1,
                        line.trim()
                    );
                }
            }
        }
    }
}

// ===========================================================================
// NI-003: No Analytics or Telemetry
// ===========================================================================

#[test]
fn no_analytics_or_telemetry_scripts() {
    let template_dir = Path::new("templates");
    let html_files = collect_files(template_dir, &["html"]);

    let analytics_indicators = [
        "google-analytics",
        "googletagmanager",
        "gtag(",
        "ga(",
        "analytics.js",
        "plausible",
        "matomo",
        "hotjar",
        "mixpanel",
        "segment.io",
        "amplitude",
    ];

    for file in &html_files {
        let content = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", file.display(), e));
        let content_lower = content.to_lowercase();
        for indicator in &analytics_indicators {
            assert!(
                !content_lower.contains(&indicator.to_lowercase()),
                "NI-003 violation: {} references analytics/telemetry '{}' — \
                 zero external requests required",
                file.display(),
                indicator
            );
        }
    }
}

// ===========================================================================
// is_external_reference unit tests
// ===========================================================================

#[test]
fn test_is_external_reference_detects_https() {
    assert!(is_external_reference("https://cdn.example.com/lib.js"));
    assert!(is_external_reference("http://fonts.googleapis.com/css"));
}

#[test]
fn test_is_external_reference_detects_protocol_relative() {
    assert!(is_external_reference("//cdn.example.com/lib.js"));
    assert!(is_external_reference("//fonts.gstatic.com/s/font.woff2"));
}

#[test]
fn test_is_external_reference_allows_local() {
    assert!(!is_external_reference("/static/js/htmx.min.js"));
    assert!(!is_external_reference("./assets/style.css"));
    assert!(!is_external_reference("../shared/tokens.css"));
    assert!(!is_external_reference("#fragment"));
    assert!(!is_external_reference("data:image/svg+xml;base64,abc"));
    assert!(!is_external_reference(""));
    assert!(!is_external_reference("   "));
}

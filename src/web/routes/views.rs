//! View route handlers for the new navigation shell.
//!
//! Provides five top-level views (Overview, Charging, History, Compare, Settings)
//! with htmx partial rendering support. Each handler detects the HX-Request header
//! to serve either a full page (wrapped in shell) or a content-only partial.

use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;

use crate::web::state::AppState;

// ---------------------------------------------------------------------------
// Domain Entities
// ---------------------------------------------------------------------------

/// Five application views forming the navigation structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewId {
    Overview,
    Charging,
    History,
    Compare,
    Settings,
}

impl ViewId {
    /// Determine ViewId from request path.
    pub fn from_path(path: &str) -> Self {
        match path {
            p if p.starts_with("/charging") => Self::Charging,
            p if p.starts_with("/history") => Self::History,
            p if p.starts_with("/compare") => Self::Compare,
            p if p.starts_with("/settings") => Self::Settings,
            _ => Self::Overview,
        }
    }

    /// URL path for this view (without base_path prefix).
    pub fn path(&self) -> &'static str {
        match self {
            Self::Overview => "/overview",
            Self::Charging => "/charging",
            Self::History => "/history",
            Self::Compare => "/compare",
            Self::Settings => "/settings",
        }
    }

    /// Human-readable label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Charging => "Charging",
            Self::History => "History",
            Self::Compare => "Compare",
            Self::Settings => "Settings",
        }
    }

    /// Kebab-case slug (path without leading slash) for IDs and test attributes.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Charging => "charging",
            Self::History => "history",
            Self::Compare => "compare",
            Self::Settings => "settings",
        }
    }

    /// Inline SVG icon for the tab.
    pub fn icon_svg(&self) -> &'static str {
        match self {
            Self::Overview => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9z"/><polyline points="9,22 9,12 15,12 15,22"/></svg>"#,
            Self::Charging => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13,2 3,14 12,14 11,22 21,10 12,10 13,2"/></svg>"#,
            Self::History => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="22,12 18,12 15,21 9,3 6,12 2,12"/></svg>"#,
            Self::Compare => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>"#,
            Self::Settings => r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 00.33 1.82l.06.06a2 2 0 010 2.83 2 2 0 01-2.83 0l-.06-.06a1.65 1.65 0 00-1.82-.33 1.65 1.65 0 00-1 1.51V21a2 2 0 01-4 0v-.09a1.65 1.65 0 00-1.08-1.51 1.65 1.65 0 00-1.82.33l-.06.06a2 2 0 01-2.83-2.83l.06-.06a1.65 1.65 0 00.33-1.82 1.65 1.65 0 00-1.51-1H3a2 2 0 010-4h.09a1.65 1.65 0 001.51-1.08 1.65 1.65 0 00-.33-1.82l-.06-.06a2 2 0 012.83-2.83l.06.06a1.65 1.65 0 001.82.33H9a1.65 1.65 0 001-1.51V3a2 2 0 014 0v.09a1.65 1.65 0 001.08 1.51 1.65 1.65 0 001.82-.33l.06-.06a2 2 0 012.83 2.83l-.06.06a1.65 1.65 0 00-.33 1.82V9a1.65 1.65 0 001.51 1H21a2 2 0 010 4h-.09a1.65 1.65 0 00-1.51 1.08z"/></svg>"#,
        }
    }

    /// All views in tab-bar order.
    pub fn all() -> &'static [ViewId] {
        &[
            Self::Overview,
            Self::Charging,
            Self::History,
            Self::Compare,
            Self::Settings,
        ]
    }
}

/// Theme preference — dark is the default per BR-THEME-001.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    /// Parse from cookie value string.
    pub fn from_cookie(value: &str) -> Self {
        match value.trim() {
            "light" => Self::Light,
            _ => Self::Dark,
        }
    }

    /// CSS data-theme attribute value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// Tab item for template rendering.
pub struct TabItem {
    pub view: ViewId,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

/// Full-page shell template wrapping view content.
#[derive(Template)]
#[template(path = "shell.html")]
pub struct ShellTemplate<'a> {
    pub theme: &'a str,
    pub active_view: ViewId,
    pub content: String,
    pub base_path: &'a str,
    pub version: &'static str,
    pub tabs: Vec<TabItem>,
}

/// Overview partial (walking skeleton).
#[derive(Template)]
#[template(path = "views/overview.html")]
pub struct OverviewPartial;

/// Charging partial (walking skeleton).
#[derive(Template)]
#[template(path = "views/charging.html")]
pub struct ChargingPartial;

/// History partial (walking skeleton).
#[derive(Template)]
#[template(path = "views/history.html")]
pub struct HistoryPartial;

/// Compare partial (walking skeleton).
#[derive(Template)]
#[template(path = "views/compare.html")]
pub struct ComparePartial;

/// Settings partial with theme toggle and QR code.
#[derive(Template)]
#[template(path = "views/settings.html")]
pub struct SettingsPartial<'a> {
    pub theme: &'a str,
    pub qr_svg: &'a str,
    pub has_key: bool,
    pub version: &'static str,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const VERSION: &str = env!("CARGO_PKG_VERSION");
const THEME_COOKIE: &str = "evcc_theme";

/// Extract theme from request cookies.
fn extract_theme(headers: &HeaderMap) -> Theme {
    headers
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                c.strip_prefix(&format!("{THEME_COOKIE}="))
                    .map(|val| Theme::from_cookie(val))
            })
        })
        .unwrap_or_default()
}

/// Check if this is an htmx partial request.
fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.get("hx-request").is_some()
}

/// Build tabs with active state.
fn build_tabs(active: ViewId) -> Vec<TabItem> {
    ViewId::all()
        .iter()
        .map(|&view| TabItem {
            view,
            is_active: view == active,
        })
        .collect()
}

/// Render a full page or partial depending on HX-Request header.
fn render_view(
    headers: &HeaderMap,
    base_path: &str,
    view_id: ViewId,
    partial_html: String,
) -> Html<String> {
    if is_htmx_request(headers) {
        // Return partial + OOB tab bar update
        let oob_tabs = render_oob_tabs(base_path, view_id);
        Html(format!("{partial_html}{oob_tabs}"))
    } else {
        let theme = extract_theme(headers);
        let tmpl = ShellTemplate {
            theme: theme.as_str(),
            active_view: view_id,
            content: partial_html,
            base_path,
            version: VERSION,
            tabs: build_tabs(view_id),
        };
        Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
    }
}

/// Generate OOB swap for tab bar active state update.
fn render_oob_tabs(base_path: &str, active: ViewId) -> String {
    let mut html = String::from(
        r#"<nav id="tab-bar" class="tab-bar" role="tablist" aria-label="Main navigation" hx-swap-oob="true">"#,
    );
    for &view in ViewId::all() {
        let is_active = view == active;
        let href = format!("{}{}", base_path, view.path());
        html.push_str(&format!(
            r#"<a href="{href}" role="tab" aria-selected="{selected}" tabindex="{tabindex}" class="tab-item" hx-get="{href}" hx-target="#view-content" hx-push-url="true" data-testid="tab-{slug}"><span class="tab-icon">{icon}</span><span class="tab-label">{label}</span></a>"#,
            href = href,
            selected = is_active,
            tabindex = if is_active { "0" } else { "-1" },
            slug = view.slug(),
            icon = view.icon_svg(),
            label = view.label(),
        ));
    }
    html.push_str("</nav>");
    html
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

pub async fn overview(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let partial = OverviewPartial.render().unwrap_or_default();
    render_view(&headers, &state.config.server.base_path, ViewId::Overview, partial)
}

pub async fn charging(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let partial = ChargingPartial.render().unwrap_or_default();
    render_view(&headers, &state.config.server.base_path, ViewId::Charging, partial)
}

pub async fn history_view(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let partial = HistoryPartial.render().unwrap_or_default();
    render_view(&headers, &state.config.server.base_path, ViewId::History, partial)
}

pub async fn compare(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let partial = ComparePartial.render().unwrap_or_default();
    render_view(&headers, &state.config.server.base_path, ViewId::Compare, partial)
}

pub async fn settings(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let key = &state.config.auth.password;
    let has_key = !key.is_empty();

    let qr_svg = if has_key {
        let url = build_settings_url(&headers, &state.config.server.base_path, &state.config.server.host, state.config.server.port);
        let payload = serde_json::json!({
            "url": url,
            "key": key,
        });
        match qrcode::QrCode::new(payload.to_string().as_bytes()) {
            Ok(code) => code
                .render::<qrcode::render::svg::Color>()
                .min_dimensions(200, 200)
                .quiet_zone(true)
                .build(),
            Err(_) => String::from("<p>Failed to generate QR code</p>"),
        }
    } else {
        String::new()
    };

    let theme = extract_theme(&headers);
    let partial = SettingsPartial {
        theme: theme.as_str(),
        qr_svg: &qr_svg,
        has_key,
        version: VERSION,
    }
    .render()
    .unwrap_or_default();

    render_view(&headers, &state.config.server.base_path, ViewId::Settings, partial)
}

/// Build the external URL for QR code pairing (reused from old settings).
fn build_settings_url(headers: &HeaderMap, base_path: &str, host: &str, port: u16) -> String {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok());
    let forwarded_host = headers
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("host").and_then(|v| v.to_str().ok()));

    match (proto, forwarded_host) {
        (Some(proto), Some(fwd_host)) => format!("{proto}://{fwd_host}{base_path}"),
        (None, Some(fwd_host)) => format!("https://{fwd_host}{base_path}"),
        _ => {
            if base_path.is_empty() {
                format!("http://{host}:{port}")
            } else {
                format!("http://{host}:{port}{base_path}")
            }
        }
    }
}

/// Root redirect: / -> /overview
pub async fn root_redirect(State(state): State<AppState>) -> axum::response::Redirect {
    let base = &state.config.server.base_path;
    axum::response::Redirect::to(&format!("{base}/overview"))
}

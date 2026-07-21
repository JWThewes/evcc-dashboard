//! View route handlers for the new navigation shell.
//!
//! Provides five top-level views (Overview, Charging, History, Compare, Settings)
//! with htmx partial rendering support. Each handler detects the HX-Request header
//! to serve either a full page (wrapped in shell) or a content-only partial.

use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;

use crate::db;
use crate::model::LoadpointState;
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
// Views Domain Entities
// ---------------------------------------------------------------------------

/// Direction of a delta indicator (up, down, or neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaDirection {
    Up,
    Down,
    Neutral,
}

impl DeltaDirection {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Up => "delta-up",
            Self::Down => "delta-down",
            Self::Neutral => "delta-neutral",
        }
    }

    pub fn arrow_svg(&self) -> &'static str {
        match self {
            Self::Up => r#"<svg class="delta-arrow" viewBox="0 0 12 12" aria-hidden="true"><path d="M6 2l4 5H2z" fill="currentColor"/></svg>"#,
            Self::Down => r#"<svg class="delta-arrow" viewBox="0 0 12 12" aria-hidden="true"><path d="M6 10l4-5H2z" fill="currentColor"/></svg>"#,
            Self::Neutral => "",
        }
    }
}

/// Visual indicator for a percentage change vs yesterday.
#[derive(Debug, Clone)]
pub struct DeltaIndicator {
    pub percent: f64,
    pub direction: DeltaDirection,
    pub aria_label: String,
}

impl DeltaIndicator {
    pub fn from_percent(pct: f64) -> Self {
        let direction = if pct > 0.5 {
            DeltaDirection::Up
        } else if pct < -0.5 {
            DeltaDirection::Down
        } else {
            DeltaDirection::Neutral
        };
        let aria_label = match direction {
            DeltaDirection::Up => format!("{:.0}% more than yesterday", pct),
            DeltaDirection::Down => format!("{:.0}% less than yesterday", pct.abs()),
            DeltaDirection::Neutral => "Same as yesterday".to_string(),
        };
        Self {
            percent: pct,
            direction,
            aria_label,
        }
    }

    pub fn percent_display(&self) -> String {
        if self.direction == DeltaDirection::Neutral {
            "0%".to_string()
        } else {
            format!("{:+.0}%", self.percent)
        }
    }
}

/// A single summary card on the Overview view.
#[derive(Debug, Clone)]
pub struct SummaryCard {
    pub id: &'static str,
    pub label: &'static str,
    pub value: String,
    pub icon_svg: &'static str,
    pub delta: Option<DeltaIndicator>,
}

/// State of a loadpoint card on the Charging view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadpointCardState {
    Active,
    Idle,
    Disconnected,
}

impl LoadpointCardState {
    pub fn css_class(&self) -> &'static str {
        match self {
            Self::Active => "lp-active",
            Self::Idle => "lp-idle",
            Self::Disconnected => "lp-disconnected",
        }
    }

    pub fn status_text(&self) -> &'static str {
        match self {
            Self::Active => "Charging",
            Self::Idle => "Connected — waiting",
            Self::Disconnected => "No vehicle",
        }
    }

    /// Derive state from loadpoint booleans.
    pub fn from_loadpoint(lp: &LoadpointState) -> Self {
        let connected = lp.connected.unwrap_or(false);
        let charging = lp.charging.unwrap_or(false);
        if connected && charging {
            Self::Active
        } else if connected {
            Self::Idle
        } else {
            Self::Disconnected
        }
    }
}

/// Display-ready loadpoint card data.
#[derive(Debug, Clone)]
pub struct LoadpointCard {
    pub id: u32,
    pub title: String,
    pub power_display: String,
    pub session_energy_display: String,
    pub mode: String,
    pub vehicle_soc: Option<String>,
    pub vehicle_name: Option<String>,
    pub state: LoadpointCardState,
}

impl LoadpointCard {
    /// Transform a LoadpointState into a display-ready card.
    pub fn from_loadpoint(lp: &LoadpointState) -> Self {
        let power_display = match lp.charge_power {
            Some(w) if w >= 1000.0 => format!("{:.1} kW", w / 1000.0),
            Some(w) => format!("{:.0} W", w),
            None => "--".to_string(),
        };

        let session_energy_display = match lp.charged_energy {
            Some(wh) => {
                let kwh = wh / 1000.0;
                if kwh >= 100.0 {
                    format!("{:.0} kWh", kwh)
                } else {
                    format!("{:.1} kWh", kwh)
                }
            }
            None => "-- kWh".to_string(),
        };

        let vehicle_soc = lp
            .vehicle_soc
            .filter(|v| *v > 0.0)
            .map(|v| format!("{:.0}%", v));

        Self {
            id: lp.id,
            title: lp.title.clone().unwrap_or_else(|| format!("Loadpoint {}", lp.id)),
            power_display,
            session_energy_display,
            mode: lp.mode.clone().unwrap_or_else(|| "--".to_string()),
            vehicle_soc,
            vehicle_name: lp.vehicle_name.clone(),
            state: LoadpointCardState::from_loadpoint(lp),
        }
    }
}

/// Predefined chart time ranges for the History view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryRange {
    Day,
    Week,
    Month,
    Quarter,
}

impl HistoryRange {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Day => "24h",
            Self::Week => "7d",
            Self::Month => "30d",
            Self::Quarter => "90d",
        }
    }

    pub fn resolution(&self) -> &'static str {
        match self {
            Self::Day => "5m",
            Self::Week | Self::Month => "1h",
            Self::Quarter => "daily",
        }
    }

    pub fn seconds(&self) -> i64 {
        match self {
            Self::Day => 86_400,
            Self::Week => 604_800,
            Self::Month => 2_592_000,
            Self::Quarter => 7_776_000,
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::Day => "24h",
            Self::Week => "7d",
            Self::Month => "30d",
            Self::Quarter => "90d",
        }
    }

    pub fn all() -> &'static [HistoryRange] {
        &[Self::Day, Self::Week, Self::Month, Self::Quarter]
    }

    pub fn is_default(&self) -> bool {
        *self == Self::Week
    }
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

/// Overview partial with energy flow, self-sufficiency hero, and summary cards.
#[derive(Template)]
#[template(path = "views/overview.html")]
pub struct OverviewPartial {
    pub energy_flow_html: String,
    pub self_sufficiency_pct: Option<u8>,
    pub summary_cards: Vec<SummaryCard>,
    pub has_battery: bool,
    pub has_ev: bool,
    pub base_path: String,
}

/// Charging partial with loadpoint cards.
#[derive(Template)]
#[template(path = "views/charging.html")]
pub struct ChargingPartial {
    pub loadpoints: Vec<LoadpointCard>,
    pub base_path: String,
}

/// History partial with range selector and chart container.
#[derive(Template)]
#[template(path = "views/history.html")]
pub struct HistoryPartial {
    pub ranges: &'static [HistoryRange],
    pub chart_url: String,
    pub base_path: String,
}

/// Compare partial (walking skeleton — content owned by comparison-feature unit).
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

/// OOB partial for overview summary cards refresh.
#[derive(Template)]
#[template(path = "partials/overview_summary.html")]
pub struct OverviewSummaryPartial {
    pub summary_cards: Vec<SummaryCard>,
    pub has_battery: bool,
    pub has_ev: bool,
}

/// OOB partial for charging loadpoint cards refresh.
#[derive(Template)]
#[template(path = "partials/charging_cards.html")]
pub struct ChargingCardsPartial {
    pub loadpoints: Vec<LoadpointCard>,
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

/// Compute self-sufficiency percentage from power values.
fn compute_self_sufficiency(home_power: Option<f64>, grid_power: Option<f64>) -> Option<u8> {
    let home = home_power.filter(|v| *v > 10.0)?;
    let grid_import = grid_power.unwrap_or(0.0).max(0.0);
    let pct = ((home - grid_import) / home * 100.0).clamp(0.0, 100.0);
    Some(pct as u8)
}

/// Format Wh as a human-readable string.
fn format_wh(wh: f64) -> String {
    if wh >= 1_000_000.0 {
        format!("{:.1} MWh", wh / 1_000_000.0)
    } else if wh >= 1000.0 {
        format!("{:.1} kWh", wh / 1000.0)
    } else {
        format!("{:.0} Wh", wh)
    }
}

/// Build summary cards from today's energy totals and yesterday deltas.
fn build_summary_cards(
    totals: &crate::model::EnergyTotals,
    deltas: &[db::compare::DeltaValue],
    has_battery: bool,
    has_ev: bool,
) -> Vec<SummaryCard> {
    let find_delta = |metric: &str| -> Option<DeltaIndicator> {
        deltas
            .iter()
            .find(|d| d.metric == metric)
            .map(|d| DeltaIndicator::from_percent(d.delta_pct))
    };

    let mut cards = vec![
        SummaryCard {
            id: "solar",
            label: "Solar Production",
            value: format_wh(totals.pv_production_wh.unwrap_or(0.0)),
            icon_svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>"#,
            delta: find_delta("solar"),
        },
        SummaryCard {
            id: "consumption",
            label: "Home Consumption",
            value: format_wh(totals.home_consumption_wh.unwrap_or(0.0)),
            icon_svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9z"/></svg>"#,
            delta: find_delta("consumption"),
        },
        SummaryCard {
            id: "grid",
            label: "Grid Import",
            value: format_wh(totals.grid_import_wh.unwrap_or(0.0)),
            icon_svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="2" width="20" height="20" rx="2"/><line x1="2" y1="8" x2="22" y2="8"/><line x1="2" y1="14" x2="22" y2="14"/><line x1="8" y1="2" x2="8" y2="22"/><line x1="14" y1="2" x2="14" y2="22"/></svg>"#,
            delta: find_delta("grid_import"),
        },
    ];

    if has_battery {
        cards.push(SummaryCard {
            id: "battery",
            label: "Battery",
            value: format_wh(totals.battery_discharge_wh.unwrap_or(0.0)),
            icon_svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="1" y="6" width="18" height="12" rx="2"/><line x1="23" y1="10" x2="23" y2="14"/></svg>"#,
            delta: None,
        });
    }

    if has_ev {
        cards.push(SummaryCard {
            id: "ev-charging",
            label: "EV Charging",
            value: format_wh(0.0), // EV energy from loadpoint_samples not in EnergyTotals
            icon_svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13,2 3,14 12,14 11,22 21,10 12,10 13,2"/></svg>"#,
            delta: find_delta("ev_charging"),
        });
    }

    cards
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

pub async fn overview(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let current = state.current_state.read().await;
    let site = &current.site;

    // Compute self-sufficiency from live power values
    let self_sufficiency_pct = compute_self_sufficiency(site.home_power, site.grid_power);

    // Check conditional card visibility
    let has_battery = site.battery_soc.is_some() || site.battery_power.map_or(false, |v| v.abs() > 10.0);
    let has_ev = site.ev_connected || site.ev_power.map_or(false, |v| v > 10.0);

    // Render energy flow SVG
    let energy_flow_html =
        super::energy_flow::render_energy_flow_svg(&site, &state.config.server.base_path);
    drop(current);

    // Query today's energy totals
    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;
    let (totals, deltas) = tokio::task::spawn_blocking(move || {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return (crate::model::EnergyTotals::default(), Vec::new()),
        };
        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let now_ts = now.timestamp();
        let totals = db::query::query_energy_totals(&conn, today_start, now_ts, interval)
            .unwrap_or_default();
        let deltas = db::compare::query_yesterday_comparison(&conn, interval)
            .unwrap_or_default();
        (totals, deltas)
    })
    .await
    .unwrap_or_else(|_| (crate::model::EnergyTotals::default(), Vec::new()));

    let summary_cards = build_summary_cards(&totals, &deltas, has_battery, has_ev);

    let partial = OverviewPartial {
        energy_flow_html,
        self_sufficiency_pct,
        summary_cards,
        has_battery,
        has_ev,
        base_path: state.config.server.base_path.clone(),
    }
    .render()
    .unwrap_or_default();

    render_view(
        &headers,
        &state.config.server.base_path,
        ViewId::Overview,
        partial,
    )
}

pub async fn charging(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointCard> = current
        .loadpoints
        .values()
        .map(LoadpointCard::from_loadpoint)
        .collect();
    drop(current);
    loadpoints.sort_by_key(|lp| lp.id);

    let partial = ChargingPartial {
        loadpoints,
        base_path: state.config.server.base_path.clone(),
    }
    .render()
    .unwrap_or_default();

    render_view(&headers, &state.config.server.base_path, ViewId::Charging, partial)
}

pub async fn history_view(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let base = &state.config.server.base_path;
    let default_range = HistoryRange::Week;
    let now = chrono::Utc::now().timestamp();
    let from = now - default_range.seconds();
    let chart_url = format!(
        "{}/api/chart/energy?from={}&to={}&resolution={}",
        base, from, now, default_range.resolution()
    );

    let partial = HistoryPartial {
        ranges: HistoryRange::all(),
        chart_url,
        base_path: base.clone(),
    }
    .render()
    .unwrap_or_default();

    render_view(&headers, base, ViewId::History, partial)
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

// ---------------------------------------------------------------------------
// Partial Endpoints (htmx polling)
// ---------------------------------------------------------------------------

/// Overview summary cards partial — polled every 60 seconds.
pub async fn overview_summary_partial(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let has_battery = current.site.battery_soc.is_some()
        || current.site.battery_power.map_or(false, |v| v.abs() > 10.0);
    let has_ev = current.site.ev_connected
        || current.site.ev_power.map_or(false, |v| v > 10.0);
    drop(current);

    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;
    let (totals, deltas) = tokio::task::spawn_blocking(move || {
        let conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return (crate::model::EnergyTotals::default(), Vec::new()),
        };
        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let now_ts = now.timestamp();
        let totals = db::query::query_energy_totals(&conn, today_start, now_ts, interval)
            .unwrap_or_default();
        let deltas = db::compare::query_yesterday_comparison(&conn, interval)
            .unwrap_or_default();
        (totals, deltas)
    })
    .await
    .unwrap_or_else(|_| (crate::model::EnergyTotals::default(), Vec::new()));

    let summary_cards = build_summary_cards(&totals, &deltas, has_battery, has_ev);

    let tmpl = OverviewSummaryPartial {
        summary_cards,
        has_battery,
        has_ev,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("<!-- render error: {e} -->")))
}

/// Charging loadpoint cards partial — polled every 3 seconds.
pub async fn charging_cards_partial(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointCard> = current
        .loadpoints
        .values()
        .map(LoadpointCard::from_loadpoint)
        .collect();
    drop(current);
    loadpoints.sort_by_key(|lp| lp.id);

    let tmpl = ChargingCardsPartial { loadpoints };
    Html(tmpl.render().unwrap_or_else(|e| format!("<!-- render error: {e} -->")))
}

// ---------------------------------------------------------------------------
// Utility Handlers
// ---------------------------------------------------------------------------

/// Build the external URL for QR code pairing.
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

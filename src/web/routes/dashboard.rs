use askama::Template;
use axum::extract::State;
use axum::response::Html;

use crate::model::{LoadpointState, SiteState};
use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub base_path: String,
    pub site: SiteState,
    pub loadpoints: Vec<LoadpointState>,
}

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let current = state.current_state.read().await;
    let mut loadpoints: Vec<LoadpointState> = current.loadpoints.values().cloned().collect();
    loadpoints.sort_by_key(|lp| lp.id);

    let tmpl = DashboardTemplate {
        base_path: state.config.server.base_path.clone(),
        site: current.site.clone(),
        loadpoints,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Template)]
#[template(path = "history.html")]
pub struct HistoryTemplate {
    pub base_path: String,
}

pub async fn history(State(state): State<AppState>) -> Html<String> {
    let tmpl = HistoryTemplate {
        base_path: state.config.server.base_path.clone(),
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

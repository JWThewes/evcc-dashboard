use axum::{
    extract::Request,
    http::{StatusCode, Uri},
    middleware::Next,
    response::Response,
};

use crate::web::state::AppState;

pub async fn strip_base_path(
    axum::extract::State(state): axum::extract::State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let base = &state.config.server.base_path;
    if !base.is_empty() {
        let path = req.uri().path().to_string();
        if let Some(stripped) = path.strip_prefix(base.as_str()) {
            let new_path = if stripped.is_empty() { "/" } else { stripped };
            let query = req.uri().query().map(|q| format!("?{q}")).unwrap_or_default();
            if let Ok(uri) = format!("{new_path}{query}").parse::<Uri>() {
                *req.uri_mut() = uri;
            }
        }
    }
    next.run(req).await
}

pub async fn require_api_key(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let expected = &state.config.api.key;
    if expected.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let token = &value[7..];
            if token == expected {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

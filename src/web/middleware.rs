use axum::{
    extract::Request,
    http::{StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::web::state::AppState;

const COOKIE_NAME: &str = "evcc_session";

type HmacSha256 = Hmac<Sha256>;

/// Create a signed session cookie value.
pub fn create_auth_cookie(password: &str, base_path: &str) -> String {
    let signature = sign_token(password);
    let path = if base_path.is_empty() { "/" } else { base_path };
    format!(
        "{COOKIE_NAME}={signature}; Path={path}; HttpOnly; SameSite=Strict; Max-Age=31536000"
    )
}

fn sign_token(password: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(password.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(b"evcc-dashboard-session");
    hex::encode(mac.finalize().into_bytes())
}

fn verify_cookie(cookie_header: &str, password: &str) -> bool {
    let expected = sign_token(password);
    cookie_header
        .split(';')
        .map(|c| c.trim())
        .any(|c| {
            c.strip_prefix(&format!("{COOKIE_NAME}="))
                .is_some_and(|val| val == expected)
        })
}

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

/// Middleware for browser routes: checks session cookie, redirects to /login if missing.
pub async fn require_login(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let password = &state.config.auth.password;

    // No password configured — allow access (open mode)
    if password.is_empty() {
        return next.run(req).await;
    }

    let authenticated = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|cookies| verify_cookie(cookies, password));

    if authenticated {
        next.run(req).await
    } else {
        let base = &state.config.server.base_path;
        let login_path = format!("{base}/login");
        Redirect::to(&login_path).into_response()
    }
}

/// Middleware for API routes: checks Bearer token.
pub async fn require_bearer(
    axum::extract::State(state): axum::extract::State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let password = &state.config.auth.password;
    if password.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let token = &value[7..];
            if token == password {
                Ok(next.run(req).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

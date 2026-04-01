use askama::Template;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::Html;
use qrcode::QrCode;
use qrcode::render::svg;

use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    pub base_path: String,
    pub qr_svg: String,
    pub has_key: bool,
}

pub async fn index(State(state): State<AppState>, headers: HeaderMap) -> Html<String> {
    let key = &state.config.auth.password;
    let has_key = !key.is_empty();

    let qr_svg = if has_key {
        let url = build_external_url(&headers, &state.config.server.base_path, &state.config.server.host, state.config.server.port);

        let payload = serde_json::json!({
            "url": url,
            "key": key,
        });

        match QrCode::new(payload.to_string().as_bytes()) {
            Ok(code) => code
                .render::<svg::Color>()
                .min_dimensions(256, 256)
                .quiet_zone(true)
                .build(),
            Err(_) => String::from("<p>Failed to generate QR code</p>"),
        }
    } else {
        String::new()
    };

    let tmpl = SettingsTemplate {
        base_path: state.config.server.base_path.clone(),
        qr_svg,
        has_key,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

fn build_external_url(headers: &HeaderMap, base_path: &str, host: &str, port: u16) -> String {
    let proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok());

    let forwarded_host = headers
        .get("x-forwarded-host")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("host").and_then(|v| v.to_str().ok()));

    match (proto, forwarded_host) {
        (Some(proto), Some(fwd_host)) => {
            format!("{proto}://{fwd_host}{base_path}")
        }
        (None, Some(fwd_host)) => {
            format!("https://{fwd_host}{base_path}")
        }
        _ => {
            if base_path.is_empty() {
                format!("http://{host}:{port}")
            } else {
                format!("http://{host}:{port}{base_path}")
            }
        }
    }
}

use askama::Template;
use axum::extract::State;
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

pub async fn index(State(state): State<AppState>) -> Html<String> {
    let key = &state.config.api.key;
    let has_key = !key.is_empty();

    let qr_svg = if has_key {
        let base_path = &state.config.server.base_path;
        let host = &state.config.server.host;
        let port = state.config.server.port;

        let url = if base_path.is_empty() {
            format!("http://{host}:{port}")
        } else {
            format!("http://{host}:{port}{base_path}")
        };

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

use askama::Template;
use axum::extract::State;
use axum::response::{Html, Redirect, Response, IntoResponse};
use axum::Form;
use serde::Deserialize;

use crate::web::middleware::create_auth_cookie;
use crate::web::state::AppState;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub base_path: String,
    pub error: bool,
}

pub async fn get(State(state): State<AppState>) -> Html<String> {
    let tmpl = LoginTemplate {
        base_path: state.config.server.base_path.clone(),
        error: false,
    };
    Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub password: String,
}

pub async fn post(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Response {
    let expected = &state.config.auth.password;

    if !expected.is_empty() && form.password == *expected {
        let base_path = &state.config.server.base_path;
        let cookie = create_auth_cookie(expected, base_path);
        let redirect_to = if base_path.is_empty() {
            "/".to_string()
        } else {
            base_path.clone()
        };

        (
            [("set-cookie", cookie)],
            Redirect::to(&redirect_to),
        ).into_response()
    } else {
        let tmpl = LoginTemplate {
            base_path: state.config.server.base_path.clone(),
            error: true,
        };
        Html(tmpl.render().unwrap_or_else(|e| format!("Template error: {e}")))
            .into_response()
    }
}

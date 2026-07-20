pub mod middleware;
pub mod routes;
pub mod state;

use axum::middleware as axum_mw;
use axum::routing::get;
use axum::Router;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use self::state::AppState;

pub fn build_router(state: AppState) -> Router {
    let base = &state.config.server.base_path;

    let mobile_api = Router::new()
        .route("/live", get(routes::mobile::live))
        .route("/state", get(routes::mobile::current_state))
        .route("/energy", get(routes::mobile::energy_history))
        .route("/summaries", get(routes::mobile::daily_summaries))
        .route("/loadpoints", get(routes::mobile::loadpoints_list))
        .route("/loadpoints/{id}", get(routes::mobile::loadpoint_detail))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::require_bearer,
        ));

    // New view routes (foundation unit)
    let view_routes = Router::new()
        .route("/", get(routes::views::root_redirect))
        .route("/overview", get(routes::views::overview))
        .route("/charging", get(routes::views::charging))
        .route("/history", get(routes::views::history_view))
        .route("/compare", get(routes::views::compare))
        .route("/settings", get(routes::views::settings));

    // Legacy routes preserved for backward compatibility
    let legacy_routes = Router::new()
        .route("/legacy", get(routes::dashboard::index))
        .route("/legacy/history", get(routes::dashboard::history))
        .route(
            "/legacy/settings",
            get(routes::settings::index),
        );

    let protected = view_routes
        .merge(legacy_routes)
        .route("/partials/energy-flow", get(routes::partials::energy_flow))
        .route(
            "/partials/energy-flow-svg",
            get(routes::energy_flow::energy_flow_partial),
        )
        .route("/partials/loadpoints", get(routes::partials::loadpoints))
        .route("/partials/battery", get(routes::partials::battery_status))
        .route("/partials/summary", get(routes::partials::summary_stats))
        .route("/partials/today-energy", get(routes::partials::today_energy))
        .route("/api/chart/power", get(routes::charts::power_history))
        .route("/api/chart/energy", get(routes::charts::energy_daily))
        .route("/api/chart/battery", get(routes::charts::battery_history))
        .route(
            "/api/chart/loadpoint/{id}",
            get(routes::charts::loadpoint_history),
        )
        .route(
            "/api/compare/summary",
            get(routes::compare::handle_compare_summary),
        )
        .route(
            "/api/compare/chart",
            get(routes::compare::handle_compare_chart),
        )
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::require_login,
        ));

    let api_routes = protected
        .nest("/api/mobile", mobile_api)
        .route("/login", get(routes::login::get).post(routes::login::post))
        .route("/health", get(routes::health::check))
        .nest_service("/static", ServeDir::new("static"));

    let app = if base.is_empty() {
        api_routes
    } else {
        Router::new()
            .nest(base, api_routes)
            .layer(axum_mw::from_fn_with_state(
                state.clone(),
                middleware::strip_base_path,
            ))
    };

    app.layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

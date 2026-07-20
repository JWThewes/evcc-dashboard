//! Compare API route handlers.
//!
//! Implements two JSON endpoints for period-over-period energy comparison:
//! - GET /api/compare/summary — aggregated energy totals for current vs previous period
//! - GET /api/compare/chart — time-series data for overlay chart rendering
//!
//! All routes are protected by cookie-based auth middleware (require_login).
//! Per dec-comparison-api-separate-endpoints, summary and chart are separate endpoints.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::db::compare;
use crate::db::compare::{PeriodSummary, TimeValue};
use crate::web::state::AppState;

// ---------------------------------------------------------------------------
// Domain Entities
// ---------------------------------------------------------------------------

/// Supported comparison window sizes.
/// Deserialized from lowercase query parameter values.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompareWindow {
    Day,
    Week,
    Month,
}

impl CompareWindow {
    /// Returns the resolution string for chart query grouping.
    pub fn resolution(&self) -> &'static str {
        match self {
            Self::Day => "5m",
            Self::Week => "1h",
            Self::Month => "daily",
        }
    }

    /// Compute period boundaries as Unix timestamps.
    /// Implements the Period Resolution Algorithm from the business logic model.
    pub fn compute_periods(&self) -> PeriodBounds {
        let now = Utc::now();
        let now_ts = now.timestamp();

        match self {
            Self::Day => {
                let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
                let elapsed = now_ts - today_start;
                let yesterday_start = today_start - 86400;
                let yesterday_end = yesterday_start + elapsed;

                PeriodBounds {
                    current_start: today_start,
                    current_end: now_ts,
                    previous_start: yesterday_start,
                    previous_end: yesterday_end,
                }
            }
            Self::Week => {
                let today = now.date_naive();
                let days_since_monday = today.weekday().num_days_from_monday() as i64;
                let monday = today - chrono::Duration::days(days_since_monday);
                let current_start = monday.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
                let elapsed = now_ts - current_start;
                let previous_start = current_start - 7 * 86400;
                let previous_end = previous_start + elapsed;

                PeriodBounds {
                    current_start,
                    current_end: now_ts,
                    previous_start,
                    previous_end,
                }
            }
            Self::Month => {
                let today = now.date_naive();
                let first_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
                let current_start = first_of_month.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
                let elapsed = now_ts - current_start;

                // Previous month first day
                let prev_month = if today.month() == 1 {
                    NaiveDate::from_ymd_opt(today.year() - 1, 12, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(today.year(), today.month() - 1, 1).unwrap()
                };
                let previous_start = prev_month.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();

                // Clamp previous_end: same elapsed offset but not beyond end of previous month
                let prev_month_end = first_of_month.pred_opt().unwrap()
                    .and_hms_opt(23, 59, 59).unwrap().and_utc().timestamp();
                let previous_end = (previous_start + elapsed).min(prev_month_end);

                PeriodBounds {
                    current_start,
                    current_end: now_ts,
                    previous_start,
                    previous_end,
                }
            }
        }
    }
}

/// Four energy metrics supported by the chart endpoint.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompareMetric {
    Solar,
    Consumption,
    GridImport,
    GridExport,
}

impl CompareMetric {
    /// SQL CASE expression for extracting this metric from energy_samples columns.
    pub fn sql_expression(&self) -> &'static str {
        match self {
            Self::Solar => "CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END",
            Self::Consumption => "CASE WHEN home_power > 0 THEN home_power ELSE 0 END",
            Self::GridImport => "CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END",
            Self::GridExport => "CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END",
        }
    }
}

/// Computed time boundaries for a comparison window.
#[derive(Debug, Clone, Copy)]
pub struct PeriodBounds {
    pub current_start: i64,
    pub current_end: i64,
    pub previous_start: i64,
    pub previous_end: i64,
}

impl PeriodBounds {
    /// Format start/end as date strings for daily_summaries queries.
    pub fn current_date_range(&self) -> (String, String) {
        (ts_to_date(self.current_start), ts_to_date(self.current_end))
    }

    pub fn previous_date_range(&self) -> (String, String) {
        (ts_to_date(self.previous_start), ts_to_date(self.previous_end))
    }
}

/// Summary API response containing current and previous period totals.
#[derive(Debug, Clone, Serialize)]
pub struct CompareSummaryResponse {
    pub current_period: PeriodSummary,
    pub previous_period: PeriodSummary,
}

/// Chart API response containing two time series for overlay rendering.
#[derive(Debug, Clone, Serialize)]
pub struct CompareChartResponse {
    pub current_series: Vec<TimeValue>,
    pub previous_series: Vec<TimeValue>,
}

/// Query parameters for the summary endpoint.
#[derive(Debug, Deserialize)]
pub struct CompareSummaryParams {
    pub window: CompareWindow,
}

/// Query parameters for the chart endpoint.
#[derive(Debug, Deserialize)]
pub struct CompareChartParams {
    pub window: CompareWindow,
    pub metric: CompareMetric,
}

// ---------------------------------------------------------------------------
// Error Response
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

fn bad_request(message: impl Into<String>) -> impl IntoResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "VALIDATION_FAILED",
                message: message.into(),
            },
        }),
    )
}

// ---------------------------------------------------------------------------
// Route Handlers
// ---------------------------------------------------------------------------

/// GET /api/compare/summary?window={day|week|month}
///
/// Returns aggregated energy totals for both current and previous periods.
/// Always returns 200 with zero values if data is unavailable.
pub async fn handle_compare_summary(
    State(state): State<AppState>,
    params: Result<Query<CompareSummaryParams>, axum::extract::rejection::QueryRejection>,
) -> impl IntoResponse {
    let params = match params {
        Ok(Query(p)) => p,
        Err(_) => {
            return bad_request(
                "Invalid 'window' parameter. Must be one of: day, week, month",
            )
            .into_response();
        }
    };

    let pool = state.db_pool.clone();
    let interval = state.config.sampling.interval_seconds as f64;
    let window = params.window;
    let bounds = window.compute_periods();

    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;

        let (current, previous) = match window {
            CompareWindow::Month => {
                let (cs, ce) = bounds.current_date_range();
                let (ps, pe) = bounds.previous_date_range();
                let current = compare::query_period_summary_daily(&conn, &cs, &ce)?;
                let previous = compare::query_period_summary_daily(&conn, &ps, &pe)?;
                (current, previous)
            }
            _ => {
                let current = compare::query_period_summary(
                    &conn,
                    bounds.current_start,
                    bounds.current_end,
                    interval,
                )?;
                let previous = compare::query_period_summary(
                    &conn,
                    bounds.previous_start,
                    bounds.previous_end,
                    interval,
                )?;
                (current, previous)
            }
        };

        Ok::<_, anyhow::Error>(CompareSummaryResponse {
            current_period: current,
            previous_period: previous,
        })
    })
    .await;

    match result {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(e)) => {
            tracing::error!("Compare summary query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(e) => {
            tracing::error!("Compare summary task panicked: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// GET /api/compare/chart?window={day|week|month}&metric={solar|consumption|grid_import|grid_export}
///
/// Returns time-series data points for both periods, suitable for overlay chart rendering.
/// Server returns raw timestamps; the frontend offsets previous_series for alignment.
pub async fn handle_compare_chart(
    State(state): State<AppState>,
    params: Result<Query<CompareChartParams>, axum::extract::rejection::QueryRejection>,
) -> impl IntoResponse {
    let params = match params {
        Ok(Query(p)) => p,
        Err(_) => {
            return bad_request(
                "Invalid parameters. 'window' must be day|week|month, 'metric' must be solar|consumption|grid_import|grid_export",
            )
            .into_response();
        }
    };

    let pool = state.db_pool.clone();
    let window = params.window;
    let metric = params.metric;
    let bounds = window.compute_periods();
    let resolution = window.resolution().to_string();
    let metric_sql = metric.sql_expression().to_string();

    let result = tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;

        let current_series = compare::query_period_chart(
            &conn,
            bounds.current_start,
            bounds.current_end,
            &metric_sql,
            &resolution,
        )?;

        let previous_series = compare::query_period_chart(
            &conn,
            bounds.previous_start,
            bounds.previous_end,
            &metric_sql,
            &resolution,
        )?;

        Ok::<_, anyhow::Error>(CompareChartResponse {
            current_series,
            previous_series,
        })
    })
    .await;

    match result {
        Ok(Ok(response)) => Json(response).into_response(),
        Ok(Err(e)) => {
            tracing::error!("Compare chart query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(e) => {
            tracing::error!("Compare chart task panicked: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn ts_to_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

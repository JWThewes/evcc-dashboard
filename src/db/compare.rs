//! Database query functions for the comparison feature.
//!
//! Implements period summary aggregation, chart time-series extraction,
//! and the shared yesterday-comparison contract consumed by the views unit.
//! All queries are read-only against existing tables per dec-no-schema-migration.

use rusqlite::{params, Connection};
use serde::Serialize;

// ---------------------------------------------------------------------------
// Types used by both the query layer and the route handlers.
// Defined here to avoid circular dependencies.
// ---------------------------------------------------------------------------

/// Aggregated energy totals for a single comparison period.
#[derive(Debug, Clone, Serialize)]
pub struct PeriodSummary {
    pub solar_kwh: f64,
    pub consumption_kwh: f64,
    pub grid_import_kwh: f64,
    pub grid_export_kwh: f64,
    pub ev_charging_kwh: f64,
    pub self_sufficiency_pct: f64,
}

impl PeriodSummary {
    /// Construct from raw Wh values, converting to kWh and computing self-sufficiency.
    pub fn from_wh(
        solar_wh: f64,
        consumption_wh: f64,
        grid_import_wh: f64,
        grid_export_wh: f64,
        ev_charging_wh: f64,
    ) -> Self {
        let consumption_kwh = consumption_wh / 1000.0;
        let grid_import_kwh = grid_import_wh / 1000.0;
        let self_sufficiency_pct = if consumption_kwh > 0.0 {
            (100.0 * (1.0 - grid_import_kwh / consumption_kwh)).clamp(0.0, 100.0)
        } else {
            0.0
        };

        Self {
            solar_kwh: solar_wh / 1000.0,
            consumption_kwh,
            grid_import_kwh,
            grid_export_kwh: grid_export_wh / 1000.0,
            ev_charging_kwh: ev_charging_wh / 1000.0,
            self_sufficiency_pct,
        }
    }
}

/// A single data point in a time series (chart endpoint).
#[derive(Debug, Clone, Serialize)]
pub struct TimeValue {
    pub timestamp: i64,
    pub value: f64,
}

/// Yesterday comparison delta value — the shared contract.
/// Consumed by the views unit for Overview summary card delta indicators.
#[derive(Debug, Clone, Serialize)]
pub struct DeltaValue {
    pub metric: String,
    pub current_wh: f64,
    pub previous_wh: f64,
    pub delta_pct: f64,
}

/// Aggregate energy totals from energy_samples for a given time range.
/// Returns raw Wh values (caller converts to kWh via PeriodSummary::from_wh).
/// Uses interval_seconds as the sampling interval for W→Wh conversion.
pub fn query_period_summary(
    conn: &Connection,
    start: i64,
    end: i64,
    interval_seconds: f64,
) -> anyhow::Result<PeriodSummary> {
    let factor = interval_seconds / 3600.0;

    let mut stmt = conn.prepare(
        "SELECT
            SUM(CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END),
            SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END),
            SUM(CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END),
            SUM(CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END),
            COUNT(*)
         FROM energy_samples
         WHERE timestamp >= ?1 AND timestamp < ?2",
    )?;

    let (solar_wh, consumption_wh, grid_import_wh, grid_export_wh, _count) = stmt.query_row(
        params![start, end],
        |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                row.get::<_, i64>(4)?,
            ))
        },
    )?;

    // Query EV charging from loadpoint_samples
    let ev_wh = query_ev_charging(conn, start, end, interval_seconds)?;

    Ok(PeriodSummary::from_wh(
        solar_wh * factor,
        consumption_wh * factor,
        grid_import_wh * factor,
        grid_export_wh * factor,
        ev_wh,
    ))
}

/// Aggregate energy totals from daily_summaries for a date range (month window).
/// daily_summaries already store values in Wh.
pub fn query_period_summary_daily(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> anyhow::Result<PeriodSummary> {
    let mut stmt = conn.prepare(
        "SELECT
            SUM(pv_production_wh),
            SUM(home_consumption_wh),
            SUM(grid_import_wh),
            SUM(grid_export_wh)
         FROM daily_summaries
         WHERE date >= ?1 AND date <= ?2",
    )?;

    let (solar_wh, consumption_wh, grid_import_wh, grid_export_wh) = stmt.query_row(
        params![start_date, end_date],
        |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            ))
        },
    )?;

    // EV charging for month: aggregate from loadpoint_samples using date range
    // Convert dates back to timestamps for loadpoint query
    let start_ts = date_to_timestamp(start_date).unwrap_or(0);
    let end_ts = date_to_timestamp(end_date).unwrap_or(0) + 86400; // end of day

    let ev_wh = query_ev_charging(conn, start_ts, end_ts, 5.0)?;

    Ok(PeriodSummary::from_wh(
        solar_wh,
        consumption_wh,
        grid_import_wh,
        grid_export_wh,
        ev_wh,
    ))
}

/// Aggregate EV charging energy from all loadpoints in a time range.
fn query_ev_charging(
    conn: &Connection,
    start: i64,
    end: i64,
    interval_seconds: f64,
) -> anyhow::Result<f64> {
    let factor = interval_seconds / 3600.0;

    let mut stmt = conn.prepare(
        "SELECT SUM(CASE WHEN charge_power > 0 THEN charge_power ELSE 0 END)
         FROM loadpoint_samples
         WHERE timestamp >= ?1 AND timestamp < ?2",
    )?;

    let total: f64 = stmt
        .query_row(params![start, end], |row| {
            Ok(row.get::<_, Option<f64>>(0)?.unwrap_or(0.0))
        })?;

    Ok(total * factor)
}

/// Query time-series data for chart rendering grouped by resolution.
pub fn query_period_chart(
    conn: &Connection,
    start: i64,
    end: i64,
    metric_sql: &str,
    resolution: &str,
) -> anyhow::Result<Vec<TimeValue>> {
    let (table, group_seconds) = resolve_chart_table_and_grouping(resolution);

    let sql = if resolution == "daily" {
        // For daily resolution, map metric to daily_summaries column
        let column = match metric_sql {
            s if s.contains("pv_power") => "pv_production_wh",
            s if s.contains("home_power") => "home_consumption_wh",
            s if s.contains("grid_power > 0") => "grid_import_wh",
            s if s.contains("grid_power < 0") => "grid_export_wh",
            _ => "pv_production_wh",
        };

        let start_date = timestamp_to_date(start);
        let end_date = timestamp_to_date(end);

        let mut stmt = conn.prepare(&format!(
            "SELECT date, {column}
             FROM daily_summaries
             WHERE date >= ?1 AND date <= ?2
             ORDER BY date"
        ))?;

        let mut results = Vec::new();
        let rows = stmt.query_map(params![start_date, end_date], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<f64>>(1)?))
        })?;

        for row in rows {
            let (date, value) = row?;
            if let Some(ts) = date_to_timestamp(&date) {
                results.push(TimeValue {
                    timestamp: ts,
                    value: value.unwrap_or(0.0),
                });
            }
        }

        return Ok(results);
    } else {
        format!(
            "SELECT (timestamp / {group_seconds}) * {group_seconds} as ts,
                    AVG({metric_sql}) * {interval_factor}
             FROM {table}
             WHERE timestamp >= ?1 AND timestamp < ?2
             GROUP BY ts ORDER BY ts",
            group_seconds = group_seconds,
            metric_sql = metric_sql,
            table = table,
            interval_factor = if resolution == "5m" {
                300.0 / 3600.0
            } else {
                1.0 // 1h resolution: avg W * 1h = Wh
            },
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let mut results = Vec::new();

    let rows = stmt.query_map(params![start, end], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<f64>>(1)?))
    })?;

    for row in rows {
        let (ts, value) = row?;
        results.push(TimeValue {
            timestamp: ts,
            value: value.unwrap_or(0.0),
        });
    }

    Ok(results)
}

/// Shared contract: query_yesterday_comparison
/// Returns per-metric delta percentages comparing today's elapsed energy
/// against the same elapsed time yesterday.
/// Consumed by the views unit for Overview summary card delta indicators.
pub fn query_yesterday_comparison(
    conn: &Connection,
    interval_seconds: f64,
) -> anyhow::Result<Vec<DeltaValue>> {
    let now = chrono::Utc::now().timestamp();
    let today_start = (now / 86400) * 86400; // midnight UTC today
    let yesterday_start = today_start - 86400;
    let elapsed = now - today_start;
    let yesterday_end = yesterday_start + elapsed;

    let factor = interval_seconds / 3600.0;

    let query_totals = |start: i64, end: i64| -> anyhow::Result<(f64, f64, f64, f64)> {
        let mut stmt = conn.prepare(
            "SELECT
                SUM(CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END),
                SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END),
                SUM(CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END),
                SUM(CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END)
             FROM energy_samples
             WHERE timestamp >= ?1 AND timestamp < ?2",
        )?;

        let result = stmt.query_row(params![start, end], |row| {
            Ok((
                row.get::<_, Option<f64>>(0)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(1)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            ))
        })?;

        Ok(result)
    };

    let (today_pv, today_home, today_import, today_export) = query_totals(today_start, now)?;
    let (yest_pv, yest_home, yest_import, yest_export) =
        query_totals(yesterday_start, yesterday_end)?;

    let compute_delta = |current: f64, previous: f64| -> f64 {
        if previous > 0.0 {
            ((current - previous) / previous) * 100.0
        } else if current > 0.0 {
            100.0 // infinite increase, cap at +100%
        } else {
            0.0 // both zero
        }
    };

    let metrics = vec![
        ("solar", today_pv * factor, yest_pv * factor),
        ("consumption", today_home * factor, yest_home * factor),
        ("grid_import", today_import * factor, yest_import * factor),
        ("grid_export", today_export * factor, yest_export * factor),
    ];

    Ok(metrics
        .into_iter()
        .map(|(metric, current_wh, previous_wh)| DeltaValue {
            metric: metric.to_string(),
            current_wh,
            previous_wh,
            delta_pct: compute_delta(current_wh, previous_wh),
        })
        .collect())
}

/// Map resolution string to table name and grouping interval for chart queries.
fn resolve_chart_table_and_grouping(resolution: &str) -> (&'static str, i64) {
    match resolution {
        "5m" => ("energy_samples", 300),
        "1h" => ("energy_samples_1m", 3600),
        _ => ("energy_samples", 300),
    }
}

/// Convert a date string "YYYY-MM-DD" to a Unix timestamp at midnight UTC.
fn date_to_timestamp(date: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

/// Convert a Unix timestamp to date string "YYYY-MM-DD".
fn timestamp_to_date(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

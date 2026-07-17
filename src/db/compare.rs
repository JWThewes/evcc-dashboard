//! Comparison database queries.
//!
//! Provides the `query_yesterday_comparison` function (contract-yesterday-comparison-query)
//! consumed by the views unit for Overview summary card delta indicators.

use rusqlite::{params, Connection};

/// A single metric's delta percentage comparing today vs yesterday.
#[derive(Debug, Clone)]
pub struct DeltaValue {
    /// Metric identifier (e.g., "solar", "consumption", "grid_import", "grid_export", "ev_charging").
    pub metric: String,
    /// Percentage change: positive = increase, negative = decrease. Clamped to [-100, 100].
    pub delta_pct: f64,
}

/// Query today's and yesterday's energy totals, returning per-metric delta percentages.
///
/// Contract: `contract-yesterday-comparison-query`
/// Consumed by: views unit (Overview summary cards)
///
/// Returns an empty vec on error or when data is insufficient.
pub fn query_yesterday_comparison(
    conn: &Connection,
    interval_seconds: f64,
) -> anyhow::Result<Vec<DeltaValue>> {
    let now = chrono::Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp();
    let now_ts = now.timestamp();

    // Yesterday: same time window as elapsed today
    let elapsed_today = now_ts - today_start;
    let yesterday_start = today_start - 86_400;
    let yesterday_end = yesterday_start + elapsed_today;

    let factor = interval_seconds / 3600.0; // W*samples -> Wh

    let today = query_period_totals(conn, today_start, now_ts, factor)?;
    let yesterday = query_period_totals(conn, yesterday_start, yesterday_end, factor)?;

    let mut deltas = Vec::with_capacity(5);

    deltas.push(compute_delta("solar", today.solar_wh, yesterday.solar_wh));
    deltas.push(compute_delta(
        "consumption",
        today.consumption_wh,
        yesterday.consumption_wh,
    ));
    deltas.push(compute_delta(
        "grid_import",
        today.grid_import_wh,
        yesterday.grid_import_wh,
    ));
    deltas.push(compute_delta(
        "grid_export",
        today.grid_export_wh,
        yesterday.grid_export_wh,
    ));
    deltas.push(compute_delta(
        "ev_charging",
        today.ev_charging_wh,
        yesterday.ev_charging_wh,
    ));

    Ok(deltas)
}

/// Internal period totals for comparison.
struct PeriodTotals {
    solar_wh: f64,
    consumption_wh: f64,
    grid_import_wh: f64,
    grid_export_wh: f64,
    ev_charging_wh: f64,
}

fn query_period_totals(
    conn: &Connection,
    from: i64,
    to: i64,
    factor: f64,
) -> anyhow::Result<PeriodTotals> {
    let mut stmt = conn.prepare(
        "SELECT
            COALESCE(SUM(CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END), 0),
            COALESCE(SUM(CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END), 0)
         FROM energy_samples
         WHERE timestamp >= ?1 AND timestamp < ?2",
    )?;

    let (solar, consumption, grid_import, grid_export) =
        stmt.query_row(params![from, to], |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
                row.get::<_, f64>(3)?,
            ))
        })?;

    // EV charging from loadpoint_samples
    let ev_wh = conn
        .query_row(
            "SELECT COALESCE(SUM(CASE WHEN charge_power > 0 THEN charge_power ELSE 0 END), 0)
             FROM loadpoint_samples
             WHERE timestamp >= ?1 AND timestamp < ?2",
            params![from, to],
            |row| row.get::<_, f64>(0),
        )
        .unwrap_or(0.0);

    Ok(PeriodTotals {
        solar_wh: solar * factor,
        consumption_wh: consumption * factor,
        grid_import_wh: grid_import * factor,
        grid_export_wh: grid_export * factor,
        ev_charging_wh: ev_wh * factor,
    })
}

fn compute_delta(metric: &str, today: f64, yesterday: f64) -> DeltaValue {
    let delta_pct = if yesterday == 0.0 {
        if today > 0.0 {
            100.0
        } else {
            0.0
        }
    } else {
        ((today - yesterday) / yesterday * 100.0).clamp(-100.0, 100.0)
    };

    DeltaValue {
        metric: metric.to_string(),
        delta_pct,
    }
}

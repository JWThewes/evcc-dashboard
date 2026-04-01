use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::model::ChartData;

pub fn query_power_history(
    conn: &Connection,
    from: i64,
    to: i64,
    resolution: &str,
) -> anyhow::Result<ChartData> {
    let (table, group_by) = resolve_table_and_grouping(resolution);

    let sql = if group_by > 0 {
        format!(
            "SELECT (timestamp / {group_by}) * {group_by} as ts,
                    AVG(grid_power), AVG(pv_power), AVG(home_power), AVG(battery_power)
             FROM {table}
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY ts ORDER BY ts"
        )
    } else {
        format!(
            "SELECT timestamp, grid_power, pv_power, home_power, battery_power
             FROM {table}
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp"
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let mut timestamps = Vec::new();
    let mut grid = Vec::new();
    let mut pv = Vec::new();
    let mut home = Vec::new();
    let mut battery = Vec::new();

    let rows = stmt.query_map(params![from, to], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?,
            row.get::<_, Option<f64>>(3)?,
            row.get::<_, Option<f64>>(4)?,
        ))
    })?;

    for row in rows {
        let (ts, g, p, h, b) = row?;
        timestamps.push(ts);
        grid.push(g);
        pv.push(p);
        home.push(h);
        battery.push(b);
    }

    let mut series = HashMap::new();
    series.insert("grid_power".to_string(), grid);
    series.insert("pv_power".to_string(), pv);
    series.insert("home_power".to_string(), home);
    series.insert("battery_power".to_string(), battery);

    Ok(ChartData { timestamps, series })
}

pub fn query_battery_history(
    conn: &Connection,
    from: i64,
    to: i64,
    resolution: &str,
) -> anyhow::Result<ChartData> {
    let (table, group_by) = resolve_table_and_grouping(resolution);

    let sql = if group_by > 0 {
        format!(
            "SELECT (timestamp / {group_by}) * {group_by} as ts,
                    AVG(battery_power), AVG(battery_soc)
             FROM {table}
             WHERE timestamp >= ?1 AND timestamp <= ?2
             GROUP BY ts ORDER BY ts"
        )
    } else {
        format!(
            "SELECT timestamp, battery_power, battery_soc
             FROM {table}
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp"
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let mut timestamps = Vec::new();
    let mut power = Vec::new();
    let mut soc = Vec::new();

    let rows = stmt.query_map(params![from, to], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?,
        ))
    })?;

    for row in rows {
        let (ts, p, s) = row?;
        timestamps.push(ts);
        power.push(p);
        soc.push(s);
    }

    let mut series = HashMap::new();
    series.insert("battery_power".to_string(), power);
    series.insert("battery_soc".to_string(), soc);

    Ok(ChartData { timestamps, series })
}

pub fn query_loadpoint_history(
    conn: &Connection,
    loadpoint_id: u32,
    from: i64,
    to: i64,
    resolution: &str,
) -> anyhow::Result<ChartData> {
    let (_table, group_by) = resolve_table_and_grouping(resolution);

    let sql = if group_by > 0 {
        format!(
            "SELECT (timestamp / {group_by}) * {group_by} as ts, AVG(charge_power)
             FROM loadpoint_samples
             WHERE loadpoint_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             GROUP BY ts ORDER BY ts"
        )
    } else {
        "SELECT timestamp, charge_power
         FROM loadpoint_samples
         WHERE loadpoint_id = ?1 AND timestamp >= ?2 AND timestamp <= ?3
         ORDER BY timestamp"
            .to_string()
    };

    let mut stmt = conn.prepare(&sql)?;
    let mut timestamps = Vec::new();
    let mut charge = Vec::new();

    let rows = stmt.query_map(params![loadpoint_id, from, to], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, Option<f64>>(1)?))
    })?;

    for row in rows {
        let (ts, c) = row?;
        timestamps.push(ts);
        charge.push(c);
    }

    let mut series = HashMap::new();
    series.insert("charge_power".to_string(), charge);

    Ok(ChartData { timestamps, series })
}

pub fn query_daily_chart(conn: &Connection, from: i64, to: i64) -> anyhow::Result<ChartData> {
    let from_date = chrono::DateTime::from_timestamp(from, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let to_date = chrono::DateTime::from_timestamp(to, 0)
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_default();

    let mut stmt = conn.prepare(
        "SELECT date, grid_import_wh, grid_export_wh, pv_production_wh, home_consumption_wh,
                self_sufficiency_pct
         FROM daily_summaries
         WHERE date >= ?1 AND date <= ?2
         ORDER BY date",
    )?;

    let mut timestamps = Vec::new();
    let mut grid_import = Vec::new();
    let mut grid_export = Vec::new();
    let mut pv_prod = Vec::new();
    let mut home_cons = Vec::new();
    let mut self_suff = Vec::new();

    let rows = stmt.query_map(params![from_date, to_date], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<f64>>(1)?,
            row.get::<_, Option<f64>>(2)?,
            row.get::<_, Option<f64>>(3)?,
            row.get::<_, Option<f64>>(4)?,
            row.get::<_, Option<f64>>(5)?,
        ))
    })?;

    for row in rows {
        let (date, gi, ge, pv, hc, ss) = row?;
        // Convert date string to unix timestamp for chart
        if let Ok(dt) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") {
            timestamps.push(
                dt.and_hms_opt(0, 0, 0)
                    .unwrap()
                    .and_utc()
                    .timestamp(),
            );
        }
        grid_import.push(gi);
        grid_export.push(ge);
        pv_prod.push(pv);
        home_cons.push(hc);
        self_suff.push(ss);
    }

    let mut series = HashMap::new();
    series.insert("grid_import_wh".to_string(), grid_import);
    series.insert("grid_export_wh".to_string(), grid_export);
    series.insert("pv_production_wh".to_string(), pv_prod);
    series.insert("home_consumption_wh".to_string(), home_cons);
    series.insert("self_sufficiency_pct".to_string(), self_suff);

    Ok(ChartData { timestamps, series })
}

/// Calculate energy totals for a given time range by integrating power samples.
/// Returns values in Wh. Assumes 5-second sampling interval.
pub fn query_energy_totals(
    conn: &Connection,
    from: i64,
    to: i64,
    interval_seconds: f64,
) -> anyhow::Result<EnergyTotals> {
    let mut stmt = conn.prepare(
        "SELECT
            SUM(CASE WHEN pv_power > 0 THEN pv_power ELSE 0 END),
            SUM(CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END),
            SUM(CASE WHEN grid_power < 0 THEN ABS(grid_power) ELSE 0 END),
            SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END),
            SUM(CASE WHEN battery_power > 0 THEN battery_power ELSE 0 END),
            SUM(CASE WHEN battery_power < 0 THEN ABS(battery_power) ELSE 0 END),
            COUNT(*)
         FROM energy_samples
         WHERE timestamp >= ?1 AND timestamp <= ?2",
    )?;

    let factor = interval_seconds / 3600.0; // convert W*samples to Wh

    let result = stmt.query_row(params![from, to], |row| {
        Ok(EnergyTotals {
            pv_production_wh: row.get::<_, Option<f64>>(0)?.map(|v| v * factor),
            grid_import_wh: row.get::<_, Option<f64>>(1)?.map(|v| v * factor),
            grid_export_wh: row.get::<_, Option<f64>>(2)?.map(|v| v * factor),
            home_consumption_wh: row.get::<_, Option<f64>>(3)?.map(|v| v * factor),
            // evcc: positive battery_power = discharging, negative = charging
            battery_discharge_wh: row.get::<_, Option<f64>>(4)?.map(|v| v * factor),
            battery_charge_wh: row.get::<_, Option<f64>>(5)?.map(|v| v * factor),
            sample_count: row.get(6)?,
        })
    })?;

    Ok(result)
}

use crate::model::{DailySummary, EnergyTotals};

pub fn query_daily_summaries(
    conn: &Connection,
    from_date: &str,
    to_date: &str,
) -> anyhow::Result<Vec<DailySummary>> {
    let mut stmt = conn.prepare(
        "SELECT date, grid_import_wh, grid_export_wh, pv_production_wh, home_consumption_wh,
                battery_charge_wh, battery_discharge_wh, self_sufficiency_pct
         FROM daily_summaries
         WHERE date >= ?1 AND date <= ?2
         ORDER BY date",
    )?;

    let rows = stmt.query_map(params![from_date, to_date], |row| {
        Ok(DailySummary {
            date: row.get(0)?,
            grid_import_wh: row.get(1)?,
            grid_export_wh: row.get(2)?,
            pv_production_wh: row.get(3)?,
            home_consumption_wh: row.get(4)?,
            battery_charge_wh: row.get(5)?,
            battery_discharge_wh: row.get(6)?,
            self_sufficiency_pct: row.get(7)?,
        })
    })?;

    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn query_today_pv_energy(conn: &Connection) -> anyhow::Result<Option<f64>> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let result = conn.query_row(
        "SELECT pv_production_wh FROM daily_summaries WHERE date = ?1",
        params![today],
        |row| row.get::<_, Option<f64>>(0),
    );
    match result {
        Ok(val) => Ok(val),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn resolve_table_and_grouping(resolution: &str) -> (&'static str, i64) {
    match resolution {
        "raw" => ("energy_samples", 0),
        "1m" => ("energy_samples", 60),
        "5m" => ("energy_samples_1m", 300),
        "1h" => ("energy_samples_1h", 0),
        _ => ("energy_samples", 60),
    }
}

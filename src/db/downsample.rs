use chrono::Utc;
use rusqlite::{params, Connection};

use crate::config::RetentionConfig;

pub fn run_downsample_cycle(conn: &Connection, retention: &RetentionConfig) -> anyhow::Result<()> {
    let now = Utc::now().timestamp();

    // Aggregate raw -> 1m for data older than raw_days
    let raw_cutoff = now - (retention.raw_days as i64 * 86400);
    aggregate_to_1m(conn, raw_cutoff)?;

    // Aggregate 1m -> 1h for data older than minute_days
    let minute_cutoff = now - (retention.minute_days as i64 * 86400);
    aggregate_to_1h(conn, minute_cutoff)?;

    // Delete old hourly data beyond hourly_days
    let hourly_cutoff = now - (retention.hourly_days as i64 * 86400);
    cleanup_old_data(conn, raw_cutoff, minute_cutoff, hourly_cutoff)?;

    Ok(())
}

fn aggregate_to_1m(conn: &Connection, cutoff: i64) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO energy_samples_1m (timestamp, grid_power, pv_power, home_power, battery_power, battery_soc)
         SELECT (timestamp / 60) * 60,
                AVG(grid_power), AVG(pv_power), AVG(home_power), AVG(battery_power), AVG(battery_soc)
         FROM energy_samples
         WHERE timestamp < ?1
         GROUP BY timestamp / 60",
        params![cutoff],
    )?;
    Ok(())
}

fn aggregate_to_1h(conn: &Connection, cutoff: i64) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO energy_samples_1h (timestamp, grid_power, pv_power, home_power, battery_power, battery_soc)
         SELECT (timestamp / 3600) * 3600,
                AVG(grid_power), AVG(pv_power), AVG(home_power), AVG(battery_power), AVG(battery_soc)
         FROM energy_samples_1m
         WHERE timestamp < ?1
         GROUP BY timestamp / 3600",
        params![cutoff],
    )?;
    Ok(())
}

fn cleanup_old_data(
    conn: &Connection,
    raw_cutoff: i64,
    minute_cutoff: i64,
    hourly_cutoff: i64,
) -> anyhow::Result<()> {
    // Delete raw samples that have been aggregated
    conn.execute(
        "DELETE FROM energy_samples WHERE timestamp < ?1",
        params![raw_cutoff],
    )?;

    // Delete 1m samples that have been aggregated
    conn.execute(
        "DELETE FROM energy_samples_1m WHERE timestamp < ?1",
        params![minute_cutoff],
    )?;

    // Delete very old hourly data
    conn.execute(
        "DELETE FROM energy_samples_1h WHERE timestamp < ?1",
        params![hourly_cutoff],
    )?;

    // Delete old loadpoint samples (same retention as raw)
    conn.execute(
        "DELETE FROM loadpoint_samples WHERE timestamp < ?1",
        params![raw_cutoff],
    )?;

    Ok(())
}

pub fn update_daily_summary(conn: &Connection, date: &str) -> anyhow::Result<()> {
    // Calculate daily summary from the best available resolution
    conn.execute(
        "INSERT OR REPLACE INTO daily_summaries
            (date, grid_import_wh, grid_export_wh, pv_production_wh, home_consumption_wh,
             battery_charge_wh, battery_discharge_wh, self_sufficiency_pct)
         SELECT
            ?1,
            SUM(CASE WHEN grid_power > 0 THEN grid_power * ?2 / 3600.0 ELSE 0 END),
            SUM(CASE WHEN grid_power < 0 THEN ABS(grid_power) * ?2 / 3600.0 ELSE 0 END),
            SUM(CASE WHEN pv_power > 0 THEN pv_power * ?2 / 3600.0 ELSE 0 END),
            SUM(CASE WHEN home_power > 0 THEN home_power * ?2 / 3600.0 ELSE 0 END),
            SUM(CASE WHEN battery_power > 0 THEN battery_power * ?2 / 3600.0 ELSE 0 END),
            SUM(CASE WHEN battery_power < 0 THEN ABS(battery_power) * ?2 / 3600.0 ELSE 0 END),
            CASE
                WHEN SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END) > 0
                THEN ROUND(100.0 * (1.0 - SUM(CASE WHEN grid_power > 0 THEN grid_power ELSE 0 END)
                     / SUM(CASE WHEN home_power > 0 THEN home_power ELSE 0 END)), 1)
                ELSE 0
            END
         FROM energy_samples
         WHERE timestamp >= strftime('%s', ?1) AND timestamp < strftime('%s', ?1, '+1 day')",
        params![date, 5], // 5 = sampling interval in seconds
    )?;
    Ok(())
}

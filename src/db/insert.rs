use rusqlite::{params, Connection};

use crate::model::{EnergySample, LoadpointSample};

pub fn insert_energy_sample(conn: &Connection, sample: &EnergySample) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO energy_samples (timestamp, grid_power, pv_power, home_power, battery_power, battery_soc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            sample.timestamp,
            sample.grid_power,
            sample.pv_power,
            sample.home_power,
            sample.battery_power,
            sample.battery_soc,
        ],
    )?;
    Ok(())
}

pub fn insert_loadpoint_sample(conn: &Connection, sample: &LoadpointSample) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO loadpoint_samples (timestamp, loadpoint_id, charge_power, charged_energy)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            sample.timestamp,
            sample.loadpoint_id,
            sample.charge_power,
            sample.charged_energy,
        ],
    )?;
    Ok(())
}

pub fn insert_energy_samples_batch(
    conn: &Connection,
    samples: &[EnergySample],
    loadpoint_samples: &[LoadpointSample],
) -> anyhow::Result<()> {
    let tx = conn.unchecked_transaction()?;

    for sample in samples {
        insert_energy_sample(&tx, sample)?;
    }
    for sample in loadpoint_samples {
        insert_loadpoint_sample(&tx, sample)?;
    }

    tx.commit()?;
    Ok(())
}

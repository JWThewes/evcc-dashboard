-- Raw energy samples (5s resolution)
CREATE TABLE IF NOT EXISTS energy_samples (
    timestamp INTEGER NOT NULL,
    grid_power REAL,
    pv_power REAL,
    home_power REAL,
    battery_power REAL,
    battery_soc REAL,
    PRIMARY KEY (timestamp)
) WITHOUT ROWID;

-- Loadpoint samples
CREATE TABLE IF NOT EXISTS loadpoint_samples (
    timestamp INTEGER NOT NULL,
    loadpoint_id INTEGER NOT NULL,
    charge_power REAL,
    charged_energy REAL,
    PRIMARY KEY (timestamp, loadpoint_id)
) WITHOUT ROWID;

-- Downsampled: 1-minute averages
CREATE TABLE IF NOT EXISTS energy_samples_1m (
    timestamp INTEGER NOT NULL,
    grid_power REAL,
    pv_power REAL,
    home_power REAL,
    battery_power REAL,
    battery_soc REAL,
    PRIMARY KEY (timestamp)
) WITHOUT ROWID;

-- Downsampled: 1-hour averages
CREATE TABLE IF NOT EXISTS energy_samples_1h (
    timestamp INTEGER NOT NULL,
    grid_power REAL,
    pv_power REAL,
    home_power REAL,
    battery_power REAL,
    battery_soc REAL,
    PRIMARY KEY (timestamp)
) WITHOUT ROWID;

-- Daily summaries (energy in Wh)
CREATE TABLE IF NOT EXISTS daily_summaries (
    date TEXT NOT NULL PRIMARY KEY,
    grid_import_wh REAL,
    grid_export_wh REAL,
    pv_production_wh REAL,
    home_consumption_wh REAL,
    battery_charge_wh REAL,
    battery_discharge_wh REAL,
    self_sufficiency_pct REAL
);

-- Indexes for time range queries
CREATE INDEX IF NOT EXISTS idx_energy_samples_ts ON energy_samples(timestamp);
CREATE INDEX IF NOT EXISTS idx_loadpoint_samples_ts ON loadpoint_samples(timestamp, loadpoint_id);
CREATE INDEX IF NOT EXISTS idx_energy_samples_1m_ts ON energy_samples_1m(timestamp);
CREATE INDEX IF NOT EXISTS idx_energy_samples_1h_ts ON energy_samples_1h(timestamp);

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub mqtt: MqttConfig,
    pub database: DatabaseConfig,
    pub sampling: SamplingConfig,
    pub retention: RetentionConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub api: ApiConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiConfig {
    pub key: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            key: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub base_path: String,
    #[allow(dead_code)]
    #[serde(default = "default_true")]
    pub trust_proxy_headers: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_topic_prefix")]
    pub topic_prefix: String,
    #[serde(default = "default_client_id")]
    pub client_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SamplingConfig {
    #[serde(default = "default_interval")]
    pub interval_seconds: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RetentionConfig {
    #[serde(default = "default_raw_days")]
    pub raw_days: u32,
    #[serde(default = "default_minute_days")]
    pub minute_days: u32,
    #[serde(default = "default_hourly_days")]
    pub hourly_days: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    3000
}
fn default_true() -> bool {
    true
}
fn default_mqtt_port() -> u16 {
    1883
}
fn default_topic_prefix() -> String {
    "evcc".to_string()
}
fn default_client_id() -> String {
    "evcc_dashboard".to_string()
}
fn default_db_path() -> String {
    "./data/evcc_dashboard.db".to_string()
}
fn default_interval() -> u64 {
    5
}
fn default_raw_days() -> u32 {
    7
}
fn default_minute_days() -> u32 {
    90
}
fn default_hourly_days() -> u32 {
    730
}
fn default_log_level() -> String {
    "info".to_string()
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}

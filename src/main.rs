mod config;
mod db;
mod model;
mod mqtt;
mod tasks;
mod web;

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use crate::model::CurrentState;
use crate::web::state::AppState;

#[derive(Parser)]
#[command(name = "evcc-dashboard")]
#[command(about = "Home energy monitoring dashboard")]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Load config
    let config = config::Config::load(&cli.config)?;

    // Init tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| config.logging.level.clone().into()),
        )
        .init();

    tracing::info!("Starting evcc-dashboard");

    // Init database
    let db_pool = db::create_pool(&config.database.path)?;
    tracing::info!("Database initialized at {}", config.database.path);

    // Shared state for current MQTT values
    let current_state = Arc::new(RwLock::new(CurrentState::default()));

    // MQTT setup
    let (mqtt_client, eventloop) = mqtt::create_client(&config.mqtt);

    // Channel for sending samples from MQTT to DB writer
    let (sample_tx, sample_rx) = tokio::sync::mpsc::channel(256);

    // Spawn background tasks
    let mqtt_config = config.mqtt.clone();
    let mqtt_state = current_state.clone();
    let sample_interval = std::time::Duration::from_secs(config.sampling.interval_seconds);
    tokio::spawn(async move {
        mqtt::subscriber::run_mqtt_loop(
            mqtt_client, eventloop, mqtt_config, mqtt_state, sample_tx, sample_interval,
        )
        .await;
    });

    tokio::spawn(tasks::spawn_db_writer(db_pool.clone(), sample_rx));
    tokio::spawn(tasks::spawn_downsample_task(
        db_pool.clone(),
        config.retention.clone(),
    ));
    tokio::spawn(tasks::spawn_daily_summary_task(db_pool.clone()));

    // Build web server
    let config = Arc::new(config);
    let state = AppState {
        config: config.clone(),
        db_pool,
        current_state,
    };

    let app = web::build_router(state);

    let addr = format!("{}:{}", config.server.host, config.server.port);
    tracing::info!("Listening on {addr}");
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Shutting down");
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C signal handler");
    tracing::info!("Received shutdown signal");
}

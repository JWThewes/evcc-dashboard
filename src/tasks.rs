use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::RetentionConfig;
use crate::db;
use crate::mqtt::subscriber::SampleBatch;

pub async fn spawn_db_writer(
    pool: Pool<SqliteConnectionManager>,
    mut rx: mpsc::Receiver<SampleBatch>,
) {
    while let Some(batch) = rx.recv().await {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            match pool.get() {
                Ok(conn) => {
                    if let Err(e) =
                        db::insert::insert_energy_samples_batch(&conn, &[batch.energy], &batch.loadpoints)
                    {
                        tracing::error!("Failed to insert samples: {e}");
                    }
                }
                Err(e) => tracing::error!("Failed to get DB connection: {e}"),
            }
        })
        .await
        .ok();
    }
}

pub async fn spawn_downsample_task(
    pool: Pool<SqliteConnectionManager>,
    retention: RetentionConfig,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(600)); // every 10 minutes

    loop {
        interval.tick().await;

        let pool = pool.clone();
        let retention = retention.clone();
        tokio::task::spawn_blocking(move || {
            match pool.get() {
                Ok(conn) => {
                    if let Err(e) = db::downsample::run_downsample_cycle(&conn, &retention) {
                        tracing::error!("Downsample cycle failed: {e}");
                    } else {
                        tracing::debug!("Downsample cycle completed");
                    }
                }
                Err(e) => tracing::error!("Failed to get DB connection for downsample: {e}"),
            }
        })
        .await
        .ok();
    }
}

pub async fn spawn_daily_summary_task(pool: Pool<SqliteConnectionManager>) {
    // Compute summary for yesterday on startup
    compute_yesterday_summary(&pool).await;

    // Then run at the start of every hour
    let mut interval = tokio::time::interval(Duration::from_secs(3600));

    loop {
        interval.tick().await;
        compute_yesterday_summary(&pool).await;

        // Also recompute today's running summary
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            match pool.get() {
                Ok(conn) => {
                    if let Err(e) = db::downsample::update_daily_summary(&conn, &today) {
                        tracing::error!("Failed to update today's summary: {e}");
                    }
                }
                Err(e) => tracing::error!("Failed to get DB connection: {e}"),
            }
        })
        .await
        .ok();
    }
}

async fn compute_yesterday_summary(pool: &Pool<SqliteConnectionManager>) {
    let yesterday = (Utc::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        match pool.get() {
            Ok(conn) => {
                if let Err(e) = db::downsample::update_daily_summary(&conn, &yesterday) {
                    tracing::error!("Failed to compute yesterday's summary: {e}");
                }
            }
            Err(e) => tracing::error!("Failed to get DB connection: {e}"),
        }
    })
    .await
    .ok();
}

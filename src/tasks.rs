use chrono::Utc;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::config::RetentionConfig;
use crate::db;
use crate::mqtt::subscriber::SampleBatch;
use crate::web::state::SharedPeakCache;

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

/// Background task that refreshes the 7-day peak power cache every 5 minutes.
/// On failure, retains stale cached values and retries on next cycle.
pub async fn spawn_peak_updater(
    pool: Pool<SqliteConnectionManager>,
    peak_cache: SharedPeakCache,
) {
    // Initial delay to let DB accumulate some data on fresh start
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Perform initial refresh
    refresh_peak_cache(&pool, &peak_cache).await;

    // Then refresh every 5 minutes
    let mut interval = tokio::time::interval(Duration::from_secs(300));

    loop {
        interval.tick().await;
        refresh_peak_cache(&pool, &peak_cache).await;
    }
}

async fn refresh_peak_cache(
    pool: &Pool<SqliteConnectionManager>,
    peak_cache: &SharedPeakCache,
) {
    let pool = pool.clone();
    let result = tokio::task::spawn_blocking(move || {
        match pool.get() {
            Ok(conn) => db::query::query_7d_peak_powers(&conn),
            Err(e) => Err(anyhow::anyhow!("Failed to get DB connection: {e}")),
        }
    })
    .await;

    match result {
        Ok(Ok(new_peaks)) => {
            match peak_cache.write() {
                Ok(mut cache) => {
                    *cache = new_peaks;
                    tracing::debug!("Peak cache refreshed successfully");
                }
                Err(e) => {
                    tracing::error!("Failed to acquire peak cache write lock: {e}");
                }
            }
        }
        Ok(Err(e)) => {
            tracing::warn!("Peak cache refresh query failed, retaining stale values: {e}");
        }
        Err(e) => {
            tracing::warn!("Peak cache refresh task panicked: {e}");
        }
    }
}

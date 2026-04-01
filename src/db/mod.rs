pub mod downsample;
pub mod insert;
pub mod query;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;
use std::path::Path;

pub fn create_pool(db_path: &str) -> anyhow::Result<Pool<SqliteConnectionManager>> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::builder().max_size(8).build(manager)?;

    // Enable WAL mode and other pragmas
    let conn = pool.get()?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=ON;",
    )?;

    run_migrations(&conn)?;

    Ok(pool)
}

fn run_migrations(conn: &rusqlite::Connection) -> anyhow::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS _migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
        [],
    )?;

    let applied: Vec<String> = {
        let mut stmt = conn.prepare("SELECT name FROM _migrations ORDER BY id")?;
        stmt.query_map([], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?
    };

    let mut migrations: Vec<_> = std::fs::read_dir("migrations")?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .is_some_and(|ext| ext == "sql")
        })
        .collect();
    migrations.sort_by_key(|e| e.file_name());

    for entry in migrations {
        let name = entry.file_name().to_string_lossy().to_string();
        if !applied.contains(&name) {
            let sql = std::fs::read_to_string(entry.path())?;
            tracing::info!("Applying migration: {name}");
            conn.execute_batch(&sql)?;
            conn.execute(
                "INSERT INTO _migrations (name) VALUES (?1)",
                params![name],
            )?;
        }
    }

    Ok(())
}

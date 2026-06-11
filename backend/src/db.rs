use deadpool_postgres::{Config, Pool, PoolConfig, Runtime};
use tokio_postgres::NoTls;
use crate::config::DatabaseConfig;

pub type DbPool = Pool;

pub fn create_pool(cfg: &DatabaseConfig) -> Result<DbPool, Box<dyn std::error::Error>> {
    let mut pg_cfg = Config::new();
    pg_cfg.url = Some(cfg.url.clone());

    pg_cfg.pool = Some(PoolConfig {
        max_size: cfg.pool_size,
        timeouts: deadpool_postgres::Timeouts {
            wait: Some(std::time::Duration::from_secs(cfg.wait_timeout_seconds)),
            create: Some(std::time::Duration::from_secs(30)),
            recycle: Some(std::time::Duration::from_secs(30)),
        },
        ..Default::default()
    });

    let pool = pg_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
    tracing::info!(
        "Database pool created: max_size={}, wait_timeout={}s",
        cfg.pool_size, cfg.wait_timeout_seconds
    );
    Ok(pool)
}

pub async fn init_db(pool: &DbPool) -> Result<(), Box<dyn std::error::Error>> {
    let client = pool.get().await?;

    let _rows = client.query(
        "SELECT 1 AS connection_test",
        &[]
    ).await?;

    tracing::info!("Database connection established successfully");

    Ok(())
}

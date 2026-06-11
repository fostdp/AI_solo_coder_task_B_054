mod models;
mod db;
mod config;
mod nbiot_ingest;
mod moisture_diffusion;
mod peg_penetration;
mod alert_broker;
mod handlers;
mod metrics;

use actix_web::{web, App, HttpServer, middleware};
use actix_cors::Cors;
use actix_files as fs;
use dotenvy::dotenv;
use tokio::sync::mpsc;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use metrics::Metrics;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    Metrics::init();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    info!("Starting Lacquer Monitor Backend...");

    let app_config = config::AppConfig::from_env();

    let pool = db::create_pool(&app_config.database)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

    if let Err(e) = db::init_db(&pool).await {
        tracing::error!("Database initialization failed: {}", e);
    }

    let (alert_tx, alert_rx) = mpsc::channel::<nbiot_ingest::IngestEvent>(app_config.nbiot.channel_capacity);

    let alert_broker_pool = pool.clone();
    let alert_broker_config = app_config.alert.clone();
    tokio::spawn(async move {
        let broker = alert_broker::AlertBroker::new(alert_broker_pool, alert_broker_config, alert_rx);
        broker.run().await;
    });

    let ingest_service = nbiot_ingest::NbiotIngestService::new(
        pool.clone(),
        app_config.nbiot.clone(),
        alert_tx,
    );

    let diffusion_service = moisture_diffusion::MoistureDiffusionService::new(
        app_config.diffusion.clone(),
    );

    let penetration_service = peg_penetration::PegPenetrationService::new(
        app_config.penetration.clone(),
    );

    let host = app_config.server.host.clone();
    let port = app_config.server.port;
    let workers = app_config.server.workers;
    let static_dir = app_config.server.static_dir.clone();

    info!("Server starting on {}:{} with {} workers", host, port, workers);
    info!("Serving static files from: {}", static_dir);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(ingest_service.clone()))
            .app_data(web::Data::new(diffusion_service.clone()))
            .app_data(web::Data::new(penetration_service.clone()))
            .wrap(cors)
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/api")
                    .route("/statistics", web::get().to(handlers::get_statistics))
                    .route("/lacquer-wares", web::get().to(handlers::get_lacquer_wares))
                    .route("/lacquer-wares/{id}", web::get().to(handlers::get_lacquer_ware_by_id))
                    .route("/sensors", web::get().to(handlers::get_sensors))
                    .route("/moisture/latest", web::get().to(handlers::get_latest_moisture))
                    .route("/strain/latest", web::get().to(handlers::get_latest_strain))
                    .route("/lacquer-wares/{id}/moisture", web::get().to(handlers::get_moisture_data))
                    .route("/lacquer-wares/{id}/strain", web::get().to(handlers::get_strain_data))
                    .route("/predict/moisture", web::post().to(handlers::predict_moisture_loss))
                    .route("/predict/penetration", web::post().to(handlers::predict_penetration))
                    .route("/reinforcement-agents", web::get().to(handlers::get_reinforcement_agents))
                    .route("/alerts", web::get().to(handlers::get_alerts))
                    .route("/nb-iot/data", web::post().to(handlers::receive_nb_iot_data))
                    .route("/nb-iot/batch", web::post().to(handlers::receive_nb_iot_batch))
                    .route("/metrics", web::get().to(handlers::get_metrics))
            )
            .service(
                fs::Files::new("/", &static_dir)
                    .index_file("index.html")
                    .default_handler(fs::NamedFile::open(format!("{}/index.html", static_dir)).unwrap())
            )
    })
    .workers(workers)
    .bind((host.as_str(), port))?
    .run()
    .await
}

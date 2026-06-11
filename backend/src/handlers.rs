use crate::db::DbPool;
use crate::models::*;
use crate::nbiot_ingest::NbiotIngestService;
use crate::moisture_diffusion::MoistureDiffusionService;
use crate::peg_penetration::PegPenetrationService;
use crate::dehydration_stress::DehydrationStressSolver;
use crate::convection_diffusion::ConvectionDiffusionSolver;
use crate::gpr_prediction::GaussianProcessRegressor;
use crate::dimensional_stability::DimensionalStabilitySimulator;
use crate::metrics::Metrics;
use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc, Duration};
use prometheus::Encoder;
use serde::Deserialize;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TimeRangeParams {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
}

pub async fn get_lacquer_wares(
    pool: web::Data<DbPool>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<LacquerWare>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let rows = match client.query(
        r#"
        SELECT id, name, artifact_code, description, material, excavation_site, dynasty,
               initial_moisture, current_moisture, target_moisture, status, created_at, updated_at
        FROM lacquer_ware
        ORDER BY id
        LIMIT $1 OFFSET $2
        "#,
        &[&limit, &offset],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<LacquerWare>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut lacquer_wares = Vec::new();
    for row in rows {
        lacquer_wares.push(LacquerWare {
            id: row.get("id"),
            name: row.get("name"),
            artifact_code: row.get("artifact_code"),
            description: row.get("description"),
            material: row.get("material"),
            excavation_site: row.get("excavation_site"),
            dynasty: row.get("dynasty"),
            initial_moisture: row.get("initial_moisture"),
            current_moisture: row.get("current_moisture"),
            target_moisture: row.get("target_moisture"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(lacquer_wares),
    })
}

pub async fn get_lacquer_ware_by_id(
    pool: web::Data<DbPool>,
    id: web::Path<i32>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<LacquerWare> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let row = match client.query_opt(
        r#"
        SELECT id, name, artifact_code, description, material, excavation_site, dynasty,
               initial_moisture, current_moisture, target_moisture, status, created_at, updated_at
        FROM lacquer_ware
        WHERE id = $1
        "#,
        &[&id.into_inner()],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<LacquerWare> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    match row {
        Some(row) => {
            let ware = LacquerWare {
                id: row.get("id"),
                name: row.get("name"),
                artifact_code: row.get("artifact_code"),
                description: row.get("description"),
                material: row.get("material"),
                excavation_site: row.get("excavation_site"),
                dynasty: row.get("dynasty"),
                initial_moisture: row.get("initial_moisture"),
                current_moisture: row.get("current_moisture"),
                target_moisture: row.get("target_moisture"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            HttpResponse::Ok().json(ApiResponse {
                success: true,
                message: "Success".to_string(),
                data: Some(ware),
            })
        }
        None => HttpResponse::NotFound().json(ApiResponse::<LacquerWare> {
            success: false,
            message: "Lacquer ware not found".to_string(),
            data: None,
        }),
    }
}

pub async fn get_sensors(
    pool: web::Data<DbPool>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<Sensor>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);

    let rows = match client.query(
        r#"
        SELECT id, device_id, sensor_type, lacquer_ware_id, location_on_xyz,
               installation_date, status, nb_iot_imsi, created_at
        FROM sensors
        ORDER BY id
        LIMIT $1 OFFSET $2
        "#,
        &[&limit, &offset],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<Sensor>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut sensors = Vec::new();
    for row in rows {
        sensors.push(Sensor {
            id: row.get("id"),
            device_id: row.get("device_id"),
            sensor_type: row.get("sensor_type"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            location_on_xyz: row.get("location_on_xyz"),
            installation_date: row.get("installation_date"),
            status: row.get("status"),
            nb_iot_imsi: row.get("nb_iot_imsi"),
            created_at: row.get("created_at"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(sensors),
    })
}

pub async fn get_moisture_data(
    pool: web::Data<DbPool>,
    lacquer_id: web::Path<i32>,
    query: web::Query<TimeRangeParams>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<MoistureData>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let end_time = query.end_time.unwrap_or_else(Utc::now);
    let start_time = query.start_time.unwrap_or_else(|| end_time - Duration::hours(24));

    let rows = match client.query(
        r#"
        SELECT time, sensor_id, lacquer_ware_id, moisture_content, temperature,
               raw_value, battery_level, signal_strength
        FROM moisture_data
        WHERE lacquer_ware_id = $1 AND time BETWEEN $2 AND $3
        ORDER BY time DESC
        LIMIT 1000
        "#,
        &[&lacquer_id.into_inner(), &start_time, &end_time],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<MoistureData>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut data = Vec::new();
    for row in rows {
        data.push(MoistureData {
            time: row.get("time"),
            sensor_id: row.get("sensor_id"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            moisture_content: row.get("moisture_content"),
            temperature: row.get("temperature"),
            raw_value: row.get("raw_value"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(data),
    })
}

pub async fn get_strain_data(
    pool: web::Data<DbPool>,
    lacquer_id: web::Path<i32>,
    query: web::Query<TimeRangeParams>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<StrainData>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let end_time = query.end_time.unwrap_or_else(Utc::now);
    let start_time = query.start_time.unwrap_or_else(|| end_time - Duration::hours(24));

    let rows = match client.query(
        r#"
        SELECT time, sensor_id, lacquer_ware_id, strain_value, temperature,
               raw_value, battery_level, signal_strength
        FROM strain_data
        WHERE lacquer_ware_id = $1 AND time BETWEEN $2 AND $3
        ORDER BY time DESC
        LIMIT 1000
        "#,
        &[&lacquer_id.into_inner(), &start_time, &end_time],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<StrainData>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut data = Vec::new();
    for row in rows {
        data.push(StrainData {
            time: row.get("time"),
            sensor_id: row.get("sensor_id"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            strain_value: row.get("strain_value"),
            temperature: row.get("temperature"),
            raw_value: row.get("raw_value"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(data),
    })
}

pub async fn get_latest_moisture(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<MoistureData>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let rows = match client.query(
        r#"
        SELECT DISTINCT ON (sensor_id)
            time, sensor_id, lacquer_ware_id, moisture_content, temperature,
            raw_value, battery_level, signal_strength
        FROM moisture_data
        ORDER BY sensor_id, time DESC
        "#,
        &[],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<MoistureData>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut data = Vec::new();
    for row in rows {
        data.push(MoistureData {
            time: row.get("time"),
            sensor_id: row.get("sensor_id"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            moisture_content: row.get("moisture_content"),
            temperature: row.get("temperature"),
            raw_value: row.get("raw_value"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(data),
    })
}

pub async fn get_latest_strain(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<StrainData>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let rows = match client.query(
        r#"
        SELECT DISTINCT ON (sensor_id)
            time, sensor_id, lacquer_ware_id, strain_value, temperature,
            raw_value, battery_level, signal_strength
        FROM strain_data
        ORDER BY sensor_id, time DESC
        "#,
        &[],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<StrainData>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut data = Vec::new();
    for row in rows {
        data.push(StrainData {
            time: row.get("time"),
            sensor_id: row.get("sensor_id"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            strain_value: row.get("strain_value"),
            temperature: row.get("temperature"),
            raw_value: row.get("raw_value"),
            battery_level: row.get("battery_level"),
            signal_strength: row.get("signal_strength"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(data),
    })
}

pub async fn predict_moisture_loss(
    service: web::Data<MoistureDiffusionService>,
    body: web::Json<MoisturePredictionRequest>,
) -> impl Responder {
    let mut svc = service.get_ref().clone();
    if let Some(d) = body.diffusion_coefficient {
        svc = crate::moisture_diffusion::MoistureDiffusionService::new(crate::config::DiffusionConfig {
            diffusion_coefficient: d,
            ..svc.config().clone()
        });
    }
    if let Some(t) = body.thickness {
        svc = crate::moisture_diffusion::MoistureDiffusionService::new(crate::config::DiffusionConfig {
            thickness: t,
            ..svc.config().clone()
        });
    }

    let (time_points, moisture_values, estimated_time) = svc.predict_loss(
        body.initial_moisture,
        body.target_moisture,
        body.time_hours,
        None,
    );

    let result = MoisturePredictionResult {
        time_points,
        moisture_values,
        diffusion_coefficient: svc.diffusion_coefficient(),
        estimated_dehydration_time_hours: estimated_time,
    };

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(result),
    })
}

pub async fn predict_penetration(
    service: web::Data<PegPenetrationService>,
    body: web::Json<PenetrationPredictionRequest>,
) -> impl Responder {
    let pressure_diff = body.pressure_diff;

    let (time_points, depth_values, final_depth) = service.predict(
        body.time_hours,
        pressure_diff,
        Some(body.viscosity),
        None,
    );

    let result = PenetrationPredictionResult {
        time_points,
        depth_values,
        permeability: service.permeability(),
        final_depth,
    };

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(result),
    })
}

pub async fn get_reinforcement_agents(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<ReinforcementAgent>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let rows = match client.query(
        r#"
        SELECT id, name, agent_type, concentration, viscosity, description, created_at
        FROM reinforcement_agents
        ORDER BY id
        "#,
        &[],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<ReinforcementAgent>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut agents = Vec::new();
    for row in rows {
        agents.push(ReinforcementAgent {
            id: row.get("id"),
            name: row.get("name"),
            agent_type: row.get("agent_type"),
            concentration: row.get("concentration"),
            viscosity: row.get("viscosity"),
            description: row.get("description"),
            created_at: row.get("created_at"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(agents),
    })
}

pub async fn get_alerts(
    pool: web::Data<DbPool>,
    query: web::Query<PaginationParams>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<Alert>> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let limit = query.limit.unwrap_or(50);

    let rows = match client.query(
        r#"
        SELECT id, alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold, is_acknowledged, created_at
        FROM alerts
        ORDER BY created_at DESC
        LIMIT $1
        "#,
        &[&limit],
    ).await {
        Ok(r) => r,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<Vec<Alert>> {
                success: false,
                message: format!("Query error: {}", e),
                data: None,
            }
        ),
    };

    let mut alerts = Vec::new();
    for row in rows {
        alerts.push(Alert {
            id: row.get("id"),
            alert_type: row.get("alert_type"),
            severity: row.get("severity"),
            lacquer_ware_id: row.get("lacquer_ware_id"),
            sensor_id: row.get("sensor_id"),
            message: row.get("message"),
            value: row.get("value"),
            threshold: row.get("threshold"),
            is_acknowledged: row.get("is_acknowledged"),
            created_at: row.get("created_at"),
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(alerts),
    })
}

pub async fn receive_nb_iot_data(
    ingest_service: web::Data<NbiotIngestService>,
    body: web::Json<NbIotDataPacket>,
) -> impl Responder {
    match ingest_service.ingest_single(&body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "Data received successfully".to_string(),
            data: Some("OK".to_string()),
        }),
        Err(e) if e.contains("not found") || e.contains("not assigned") || e.contains("Unknown") => {
            HttpResponse::BadRequest().json(ApiResponse::<String> {
                success: false,
                message: e,
                data: None,
            })
        }
        Err(e) if e.contains("pool") => {
            warn!("NB-IoT data rejected: {}", e);
            HttpResponse::ServiceUnavailable().json(ApiResponse::<String> {
                success: false,
                message: e,
                data: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<String> {
            success: false,
            message: e,
            data: None,
        }),
    }
}

pub async fn receive_nb_iot_batch(
    ingest_service: web::Data<NbiotIngestService>,
    body: web::Json<Vec<NbIotDataPacket>>,
) -> impl Responder {
    match ingest_service.ingest_batch(&body).await {
        Ok((success, fail)) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: format!("Batch processed: {} success, {} failed", success, fail),
            data: Some(format!("{}/{}", success, success + fail)),
        }),
        Err(e) if e.contains("Batch size") => {
            HttpResponse::BadRequest().json(ApiResponse::<String> {
                success: false,
                message: e,
                data: None,
            })
        }
        Err(e) if e.contains("pool") => {
            warn!("NB-IoT batch rejected: {}", e);
            HttpResponse::ServiceUnavailable().json(ApiResponse::<String> {
                success: false,
                message: e,
                data: None,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<String> {
            success: false,
            message: e,
            data: None,
        }),
    }
}

pub async fn get_statistics(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let client = match pool.get().await {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(
            ApiResponse::<serde_json::Value> {
                success: false,
                message: format!("Database error: {}", e),
                data: None,
            }
        ),
    };

    let mut stats = serde_json::Map::new();

    if let Ok(row) = client.query_one("SELECT COUNT(*) as count FROM lacquer_ware", &[]).await {
        stats.insert("total_lacquer_wares".to_string(), serde_json::json!(row.get::<_, i64>("count")));
    }

    if let Ok(row) = client.query_one("SELECT COUNT(*) as count FROM sensors", &[]).await {
        stats.insert("total_sensors".to_string(), serde_json::json!(row.get::<_, i64>("count")));
    }

    if let Ok(row) = client.query_one("SELECT COUNT(*) as count FROM sensors WHERE sensor_type = 'moisture'", &[]).await {
        stats.insert("moisture_sensors".to_string(), serde_json::json!(row.get::<_, i64>("count")));
    }

    if let Ok(row) = client.query_one("SELECT COUNT(*) as count FROM sensors WHERE sensor_type = 'strain'", &[]).await {
        stats.insert("strain_sensors".to_string(), serde_json::json!(row.get::<_, i64>("count")));
    }

    if let Ok(row) = client.query_one("SELECT AVG(moisture_content) as avg FROM latest_moisture_view", &[]).await {
        let avg: Option<f64> = row.get("avg");
        stats.insert("avg_moisture".to_string(), serde_json::json!(avg.unwrap_or(0.0)));
    }

    if let Ok(row) = client.query_one("SELECT AVG(strain_value) as avg FROM latest_strain_view", &[]).await {
        let avg: Option<f64> = row.get("avg");
        stats.insert("avg_strain".to_string(), serde_json::json!(avg.unwrap_or(0.0)));
    }

    if let Ok(row) = client.query_one("SELECT COUNT(*) as count FROM alerts WHERE is_acknowledged = false", &[]).await {
        stats.insert("active_alerts".to_string(), serde_json::json!(row.get::<_, i64>("count")));
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Success".to_string(),
        data: Some(serde_json::Value::Object(stats)),
    })
}

pub async fn get_metrics() -> impl Responder {
    Metrics::get().http_requests_total.inc();
    let metric_families = Metrics::gather();
    let encoder = prometheus::TextEncoder::new();
    let mut buffer = Vec::new();

    match encoder.encode(&metric_families, &mut buffer) {
        Ok(()) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4; charset=utf-8")
            .body(buffer),
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Failed to encode metrics: {}", e)),
    }
}

#[derive(Debug, Deserialize)]
pub struct StressCalculationRequest {
    pub initial_moisture: f64,
    pub target_moisture: f64,
    pub time_hours: f64,
    pub diffusion_coefficient: Option<f64>,
    pub young_modulus: Option<f64>,
    pub shrinkage_coefficient: Option<f64>,
    pub tensile_strength: Option<f64>,
    pub num_elements_x: Option<usize>,
    pub num_elements_y: Option<usize>,
}

pub async fn compute_dehydration_stress(
    _solver: web::Data<DehydrationStressSolver>,
    body: web::Json<StressCalculationRequest>,
) -> impl Responder {
    let mut config = crate::dehydration_stress::StressConfig::default();

    if let Some(e) = body.young_modulus {
        config.young_modulus = e;
    }
    if let Some(s) = body.shrinkage_coefficient {
        config.shrinkage_coefficient = s;
    }
    if let Some(t) = body.tensile_strength {
        config.tensile_strength = t;
    }
    if let Some(nx) = body.num_elements_x {
        config.num_elements_x = nx;
    }
    if let Some(ny) = body.num_elements_y {
        config.num_elements_y = ny;
    }

    let d = body.diffusion_coefficient.unwrap_or(1e-9);

    let solver = crate::dehydration_stress::DehydrationStressSolver::new(config);

    let moisture_profile = solver.generate_moisture_profile(
        body.initial_moisture,
        body.target_moisture,
        body.time_hours,
        d,
    );

    let result = solver.compute_stress_field(&moisture_profile);

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Stress calculation completed".to_string(),
        data: Some(result),
    })
}

#[derive(Debug, Deserialize)]
pub struct ConcentrationPredictionRequest {
    pub surface_concentration: f64,
    pub total_time_hours: f64,
    pub permeability: Option<f64>,
    pub viscosity: Option<f64>,
    pub diffusion_coefficient: Option<f64>,
    pub pressure_gradient: Option<f64>,
    pub porosity: Option<f64>,
    pub num_grid_x: Option<usize>,
    pub num_grid_y: Option<usize>,
}

pub async fn predict_peg_concentration(
    _solver: web::Data<ConvectionDiffusionSolver>,
    body: web::Json<ConcentrationPredictionRequest>,
) -> impl Responder {
    let mut config = crate::convection_diffusion::ConvectionDiffusionConfig::default();

    if let Some(p) = body.permeability {
        config.permeability = p;
    }
    if let Some(v) = body.viscosity {
        config.viscosity = v;
    }
    if let Some(d) = body.diffusion_coefficient {
        config.diffusion_coefficient = d;
    }
    if let Some(pg) = body.pressure_gradient {
        config.pressure_gradient = pg;
    }
    if let Some(por) = body.porosity {
        config.porosity = por;
    }
    if let Some(nx) = body.num_grid_x {
        config.num_grid_x = nx;
    }
    if let Some(ny) = body.num_grid_y {
        config.num_grid_y = ny;
    }
    config.surface_concentration = body.surface_concentration;
    config.total_time_hours = body.total_time_hours;

    let solver = crate::convection_diffusion::ConvectionDiffusionSolver::new(config);
    let result = solver.solve();

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "PEG concentration prediction completed".to_string(),
        data: Some(result),
    })
}

#[derive(Debug, Deserialize)]
pub struct GprEndpointRequest {
    pub time_hours: Vec<f64>,
    pub moisture_values: Vec<f64>,
    pub target_moisture: Option<f64>,
    pub confidence_level: Option<f64>,
    pub kernel_type: Option<String>,
    pub optimize_hyperparams: Option<bool>,
}

pub async fn predict_endpoint_gpr(
    _regressor: web::Data<GaussianProcessRegressor>,
    body: web::Json<GprEndpointRequest>,
) -> impl Responder {
    if body.time_hours.len() != body.moisture_values.len() {
        return HttpResponse::BadRequest().json(ApiResponse::<String> {
            success: false,
            message: "Time and moisture arrays must have same length".to_string(),
            data: None,
        });
    }

    if body.time_hours.len() < 2 {
        return HttpResponse::BadRequest().json(ApiResponse::<String> {
            success: false,
            message: "Need at least 2 data points".to_string(),
            data: None,
        });
    }

    let mut config = crate::gpr_prediction::GPRConfig::default();
    if let Some(t) = body.target_moisture {
        config.target_moisture = t;
    }
    if let Some(c) = body.confidence_level {
        config.confidence_level = c;
    }
    if let Some(k) = &body.kernel_type {
        config.kernel_type = k.clone();
    }

    let mut gpr = crate::gpr_prediction::GaussianProcessRegressor::new(config);

    if body.optimize_hyperparams.unwrap_or(false) {
        if let Err(e) = gpr.optimize_hyperparameters(&body.time_hours, &body.moisture_values) {
            return HttpResponse::InternalServerError().json(ApiResponse::<String> {
                success: false,
                message: format!("Hyperparameter optimization failed: {}", e),
                data: None,
            });
        }
    } else {
        if let Err(e) = gpr.fit(&body.time_hours, &body.moisture_values) {
            return HttpResponse::InternalServerError().json(ApiResponse::<String> {
                success: false,
                message: format!("GPR fitting failed: {}", e),
                data: None,
            });
        }
    }

    match gpr.predict_endpoint() {
        Ok(result) => HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "GPR endpoint prediction completed".to_string(),
            data: Some(result),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<String> {
            success: false,
            message: format!("Prediction failed: {}", e),
            data: None,
        }),
    }
}

#[derive(Debug, Deserialize)]
pub struct StabilityAssessmentRequest {
    pub initial_moisture: Option<f64>,
    pub low_moisture: Option<f64>,
    pub high_moisture: Option<f64>,
    pub agent_type: Option<String>,
    pub agent_concentration: Option<f64>,
    pub peg_molecular_weight: Option<f64>,
    pub num_cycles: Option<usize>,
    pub cycle_duration_hours: Option<f64>,
    pub compare_without_reinforcement: Option<bool>,
}

pub async fn assess_dimensional_stability(
    _simulator: web::Data<DimensionalStabilitySimulator>,
    body: web::Json<StabilityAssessmentRequest>,
) -> impl Responder {
    let mut config = crate::dimensional_stability::DimensionalStabilityConfig::default();

    if let Some(im) = body.initial_moisture {
        config.initial_moisture = im;
    }
    if let Some(lm) = body.low_moisture {
        config.low_moisture = lm;
    }
    if let Some(hm) = body.high_moisture {
        config.high_moisture = hm;
    }
    if let Some(at) = &body.agent_type {
        config.agent_type = at.clone();
    }
    if let Some(ac) = body.agent_concentration {
        config.agent_concentration = ac;
    }
    if let Some(mw) = body.peg_molecular_weight {
        config.peg_molecular_weight = mw;
    }
    if let Some(nc) = body.num_cycles {
        config.num_cycles = nc;
    }
    if let Some(cd) = body.cycle_duration_hours {
        config.cycle_duration_hours = cd;
    }

    let simulator = crate::dimensional_stability::DimensionalStabilitySimulator::new(config);
    let result = simulator.simulate();

    let mut response_data = serde_json::json!({
        "with_reinforcement": result,
    });

    if body.compare_without_reinforcement.unwrap_or(false) {
        let no_agent_result = simulator.compare_without_reinforcement();
        response_data = serde_json::json!({
            "with_reinforcement": result,
            "without_reinforcement": no_agent_result,
        });
    }

    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Dimensional stability assessment completed".to_string(),
        data: Some(response_data),
    })
}

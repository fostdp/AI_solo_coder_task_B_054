use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub diffusion: DiffusionConfig,
    pub penetration: PenetrationConfig,
    pub alert: AlertConfig,
    pub nbiot: NbIotConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
    pub static_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub pool_size: usize,
    pub wait_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffusionConfig {
    pub diffusion_coefficient: f64,
    pub thickness: f64,
    pub crusting_alpha: f64,
    pub crust_threshold: f64,
    pub default_num_points: usize,
}

impl Default for DiffusionConfig {
    fn default() -> Self {
        Self {
            diffusion_coefficient: 1e-9,
            thickness: 0.01,
            crusting_alpha: 2.0,
            crust_threshold: 30.0,
            default_num_points: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PenetrationConfig {
    pub default_permeability: f64,
    pub default_porosity: f64,
    pub default_viscosity: f64,
    pub default_pressure_diff: f64,
    pub sample_length: f64,
    pub default_num_points: usize,
}

impl Default for PenetrationConfig {
    fn default() -> Self {
        Self {
            default_permeability: 1e-14,
            default_porosity: 0.4,
            default_viscosity: 0.056,
            default_pressure_diff: 101325.0,
            sample_length: 0.05,
            default_num_points: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub moisture_drop_threshold: f64,
    pub strain_threshold: f64,
    pub check_interval_seconds: u64,
    pub sms_api_url: String,
    pub sms_api_key: String,
    pub alert_phone_number: String,
    pub satellite_api_url: String,
    pub satellite_api_key: String,
    pub satellite_recipient: String,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            moisture_drop_threshold: 10.0,
            strain_threshold: 5.0,
            check_interval_seconds: 3600,
            sms_api_url: String::new(),
            sms_api_key: String::new(),
            alert_phone_number: "13800138000".to_string(),
            satellite_api_url: String::new(),
            satellite_api_key: String::new(),
            satellite_recipient: "satellite_station_01".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NbIotConfig {
    pub max_batch_size: usize,
    pub channel_capacity: usize,
}

impl Default for NbIotConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            channel_capacity: 1024,
        }
    }
}

impl AppConfig {
    pub fn from_env() -> Self {
        let server = ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            workers: env::var("SERVER_WORKERS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or_else(num_cpus::get),
            static_dir: env::var("STATIC_DIR").unwrap_or_else(|_| "../../frontend".to_string()),
        };

        let database = DatabaseConfig {
            url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/lacquer_monitor".to_string()),
            pool_size: env::var("DB_POOL_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(16),
            wait_timeout_seconds: env::var("DB_POOL_WAIT_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        };

        let diffusion = DiffusionConfig {
            diffusion_coefficient: env::var("DIFFUSION_COEFFICIENT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1e-9),
            thickness: env::var("SAMPLE_THICKNESS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.01),
            crusting_alpha: env::var("CRUSTING_ALPHA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2.0),
            crust_threshold: env::var("CRUST_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30.0),
            default_num_points: env::var("DIFFUSION_NUM_POINTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
        };

        let penetration = PenetrationConfig {
            default_permeability: env::var("DEFAULT_PERMEABILITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1e-14),
            default_porosity: env::var("DEFAULT_POROSITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.4),
            default_viscosity: env::var("DEFAULT_VISCOSITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.056),
            default_pressure_diff: env::var("DEFAULT_PRESSURE_DIFF")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(101325.0),
            sample_length: env::var("PENETRATION_SAMPLE_LENGTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.05),
            default_num_points: env::var("PENETRATION_NUM_POINTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
        };

        let alert = AlertConfig {
            moisture_drop_threshold: env::var("MOISTURE_DROP_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0),
            strain_threshold: env::var("STRAIN_THRESHOLD")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5.0),
            check_interval_seconds: env::var("ALERT_CHECK_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            sms_api_url: env::var("SMS_API_URL").unwrap_or_default(),
            sms_api_key: env::var("SMS_API_KEY").unwrap_or_default(),
            alert_phone_number: env::var("ALERT_PHONE_NUMBER")
                .unwrap_or_else(|_| "13800138000".to_string()),
            satellite_api_url: env::var("SATELLITE_API_URL").unwrap_or_default(),
            satellite_api_key: env::var("SATELLITE_API_KEY").unwrap_or_default(),
            satellite_recipient: env::var("SATELLITE_RECIPIENT")
                .unwrap_or_else(|_| "satellite_station_01".to_string()),
        };

        let nbiot = NbIotConfig {
            max_batch_size: env::var("NBIOT_MAX_BATCH_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(100),
            channel_capacity: env::var("NBIOT_CHANNEL_CAPACITY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024),
        };

        Self { server, database, diffusion, penetration, alert, nbiot }
    }
}

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LacquerWare {
    pub id: i32,
    pub name: String,
    pub artifact_code: String,
    pub description: Option<String>,
    pub material: Option<String>,
    pub excavation_site: Option<String>,
    pub dynasty: Option<String>,
    pub initial_moisture: f64,
    pub current_moisture: Option<f64>,
    pub target_moisture: Option<f64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sensor {
    pub id: i32,
    pub device_id: String,
    pub sensor_type: String,
    pub lacquer_ware_id: Option<i32>,
    pub location_on_xyz: Option<String>,
    pub installation_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub nb_iot_imsi: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MoistureData {
    pub time: DateTime<Utc>,
    pub sensor_id: i32,
    pub lacquer_ware_id: i32,
    pub moisture_content: f64,
    pub temperature: Option<f64>,
    pub raw_value: Option<f64>,
    pub battery_level: Option<f64>,
    pub signal_strength: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrainData {
    pub time: DateTime<Utc>,
    pub sensor_id: i32,
    pub lacquer_ware_id: i32,
    pub strain_value: f64,
    pub temperature: Option<f64>,
    pub raw_value: Option<f64>,
    pub battery_level: Option<f64>,
    pub signal_strength: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReinforcementAgent {
    pub id: i32,
    pub name: String,
    pub agent_type: String,
    pub concentration: f64,
    pub viscosity: Option<f64>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PenetrationPrediction {
    pub id: i32,
    pub lacquer_ware_id: i32,
    pub agent_id: i32,
    pub prediction_time: DateTime<Utc>,
    pub depth: f64,
    pub time_hours: f64,
    pub model_params: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Alert {
    pub id: i32,
    pub alert_type: String,
    pub severity: String,
    pub lacquer_ware_id: Option<i32>,
    pub sensor_id: Option<i32>,
    pub message: Option<String>,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub is_acknowledged: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NbIotDataPacket {
    pub device_id: String,
    pub timestamp: DateTime<Utc>,
    pub sensor_type: String,
    pub value: f64,
    pub temperature: Option<f64>,
    pub battery_level: Option<f64>,
    pub signal_strength: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoisturePredictionRequest {
    pub lacquer_ware_id: i32,
    pub initial_moisture: f64,
    pub target_moisture: f64,
    pub diffusion_coefficient: Option<f64>,
    pub thickness: Option<f64>,
    pub time_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MoisturePredictionResult {
    pub time_points: Vec<f64>,
    pub moisture_values: Vec<f64>,
    pub diffusion_coefficient: f64,
    pub estimated_dehydration_time_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PenetrationPredictionRequest {
    pub lacquer_ware_id: i32,
    pub agent_id: i32,
    pub viscosity: f64,
    pub permeability: Option<f64>,
    pub pressure_diff: Option<f64>,
    pub time_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PenetrationPredictionResult {
    pub time_points: Vec<f64>,
    pub depth_values: Vec<f64>,
    pub permeability: f64,
    pub final_depth: f64,
}

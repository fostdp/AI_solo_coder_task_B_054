use crate::db::DbPool;
use crate::models::Alert;
use serde::{Deserialize, Serialize};
use std::env;
use chrono::{DateTime, Utc};
use tracing::{info, error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub moisture_drop_threshold: f64,
    pub strain_threshold: f64,
    pub check_interval_seconds: u64,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            moisture_drop_threshold: 10.0,
            strain_threshold: 5.0,
            check_interval_seconds: 3600,
        }
    }
}

pub struct AlertSystem {
    pool: DbPool,
    config: AlertConfig,
}

impl AlertSystem {
    pub fn new(pool: DbPool, config: AlertConfig) -> Self {
        Self { pool, config }
    }

    pub async fn check_moisture_alerts(&self) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
        let client = self.pool.get().await?;
        let mut alerts = Vec::new();

        let rows = client.query(
            r#"
            SELECT 
                m1.sensor_id,
                m1.lacquer_ware_id,
                m1.moisture_content as current_moisture,
                m1.time as current_time,
                m2.moisture_content as previous_moisture,
                m2.time as previous_time
            FROM moisture_data m1
            JOIN LATERAL (
                SELECT moisture_content, time
                FROM moisture_data
                WHERE sensor_id = m1.sensor_id
                  AND time < m1.time
                ORDER BY time DESC
                LIMIT 1
            ) m2 ON true
            WHERE m1.time >= NOW() - INTERVAL '2 hours'
            ORDER BY m1.time DESC
            "#,
            &[],
        ).await?;

        for row in rows {
            let sensor_id: i32 = row.get("sensor_id");
            let lacquer_ware_id: i32 = row.get("lacquer_ware_id");
            let current_moisture: f64 = row.get("current_moisture");
            let previous_moisture: f64 = row.get("previous_moisture");
            let current_time: DateTime<Utc> = row.get("current_time");
            let previous_time: DateTime<Utc> = row.get("previous_time");

            let time_diff_hours = (current_time - previous_time).num_seconds() as f64 / 3600.0;
            let moisture_drop = previous_moisture - current_moisture;
            let drop_rate_per_hour = moisture_drop / time_diff_hours.max(0.01);

            if drop_rate_per_hour > self.config.moisture_drop_threshold {
                warn!(
                    "Moisture drop alert: sensor {}, drop rate {:.2}%/h",
                    sensor_id, drop_rate_per_hour
                );

                let alert = self.create_alert(
                    "moisture_drop",
                    if drop_rate_per_hour > 20.0 { "critical" } else { "warning" },
                    Some(lacquer_ware_id),
                    Some(sensor_id),
                    format!("含水率突降: {:.2}%/小时，超过阈值 {}%/小时", 
                        drop_rate_per_hour, self.config.moisture_drop_threshold),
                    Some(drop_rate_per_hour),
                    Some(self.config.moisture_drop_threshold),
                ).await?;

                self.send_alert_notifications(&alert).await?;
                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    pub async fn check_strain_alerts(&self) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
        let client = self.pool.get().await?;
        let mut alerts = Vec::new();

        let rows = client.query(
            r#"
            SELECT DISTINCT ON (sensor_id)
                sensor_id,
                lacquer_ware_id,
                strain_value,
                time
            FROM strain_data
            WHERE time >= NOW() - INTERVAL '2 hours'
            ORDER BY sensor_id, time DESC
            "#,
            &[],
        ).await?;

        for row in rows {
            let sensor_id: i32 = row.get("sensor_id");
            let lacquer_ware_id: i32 = row.get("lacquer_ware_id");
            let strain_value: f64 = row.get("strain_value");

            if strain_value > self.config.strain_threshold {
                warn!(
                    "Strain exceed alert: sensor {}, strain {:.2}%",
                    sensor_id, strain_value
                );

                let alert = self.create_alert(
                    "strain_exceed",
                    if strain_value > 8.0 { "critical" } else { "warning" },
                    Some(lacquer_ware_id),
                    Some(sensor_id),
                    format!("收缩应变超标: {:.2}%，超过阈值 {}%", 
                        strain_value, self.config.strain_threshold),
                    Some(strain_value),
                    Some(self.config.strain_threshold),
                ).await?;

                self.send_alert_notifications(&alert).await?;
                alerts.push(alert);
            }
        }

        Ok(alerts)
    }

    async fn create_alert(
        &self,
        alert_type: &str,
        severity: &str,
        lacquer_ware_id: Option<i32>,
        sensor_id: Option<i32>,
        message: String,
        value: Option<f64>,
        threshold: Option<f64>,
    ) -> Result<Alert, Box<dyn std::error::Error>> {
        let client = self.pool.get().await?;

        let row = client.query_one(
            r#"
            INSERT INTO alerts (alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold, is_acknowledged, created_at
            "#,
            &[&alert_type, &severity, &lacquer_ware_id, &sensor_id, &message, &value, &threshold],
        ).await?;

        Ok(Alert {
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
        })
    }

    async fn send_alert_notifications(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        self.send_sms_alert(alert).await?;
        self.send_satellite_alert(alert).await?;
        Ok(())
    }

    async fn send_sms_alert(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        let phone_number = env::var("ALERT_PHONE_NUMBER").unwrap_or_else(|_| "13800138000".to_string());
        let api_url = env::var("SMS_API_URL").unwrap_or_default();
        let api_key = env::var("SMS_API_KEY").unwrap_or_default();

        let message = format!(
            "【漆器监测告警】{} - {} - {}",
            alert.severity,
            alert.alert_type,
            alert.message.as_deref().unwrap_or("未知异常")
        );

        info!("发送短信告警到 {}: {}", phone_number, message);

        if api_url.is_empty() || api_key.is_empty() {
            info!("SMS API not configured, skipping SMS notification");
            self.record_notification(alert.id, "sms", &phone_number, "skipped", "API not configured").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "phone": phone_number,
                "message": message
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = if resp.status().is_success() { "sent" } else { "failed" };
                let body = resp.text().await.unwrap_or_default();
                self.record_notification(alert.id, "sms", &phone_number, status, &body).await?;
            }
            Err(e) => {
                error!("SMS notification failed: {}", e);
                self.record_notification(alert.id, "sms", &phone_number, "failed", &e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn send_satellite_alert(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error>> {
        let recipient = env::var("SATELLITE_RECIPIENT").unwrap_or_else(|_| "satellite_station_01".to_string());
        let api_url = env::var("SATELLITE_API_URL").unwrap_or_default();
        let api_key = env::var("SATELLITE_API_KEY").unwrap_or_default();

        let message = format!(
            "LACQUER_ALERT|{}|{}|{}|{}",
            alert.id,
            alert.severity,
            alert.alert_type,
            alert.message.as_deref().unwrap_or("unknown")
        );

        info!("发送卫星通信告警到 {}: {}", recipient, message);

        if api_url.is_empty() || api_key.is_empty() {
            info!("Satellite API not configured, skipping satellite notification");
            self.record_notification(alert.id, "satellite", &recipient, "skipped", "API not configured").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&api_url)
            .header("X-API-Key", api_key)
            .json(&serde_json::json!({
                "recipient": recipient,
                "payload": message
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = if resp.status().is_success() { "sent" } else { "failed" };
                let body = resp.text().await.unwrap_or_default();
                self.record_notification(alert.id, "satellite", &recipient, status, &body).await?;
            }
            Err(e) => {
                error!("Satellite notification failed: {}", e);
                self.record_notification(alert.id, "satellite", &recipient, "failed", &e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn record_notification(
        &self,
        alert_id: i32,
        channel: &str,
        recipient: &str,
        status: &str,
        response: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.pool.get().await?;

        let sent_at = if status == "sent" { Some(Utc::now()) } else { None };

        client.execute(
            r#"
            INSERT INTO alert_notifications (alert_id, channel, recipient, status, response, sent_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            &[&alert_id, &channel, &recipient, &status, &response, &sent_at],
        ).await?;

        Ok(())
    }

    pub async fn get_recent_alerts(&self, limit: i64) -> Result<Vec<Alert>, Box<dyn std::error::Error>> {
        let client = self.pool.get().await?;

        let rows = client.query(
            r#"
            SELECT id, alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold, is_acknowledged, created_at
            FROM alerts
            ORDER BY created_at DESC
            LIMIT $1
            "#,
            &[&limit],
        ).await?;

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

        Ok(alerts)
    }
}

pub async fn run_alert_checker(pool: DbPool, config: AlertConfig) {
    let alert_system = AlertSystem::new(pool, config.clone());
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(config.check_interval_seconds));

    info!("Alert checker started, interval: {}s", config.check_interval_seconds);

    loop {
        interval.tick().await;

        info!("Running scheduled alert check...");

        match alert_system.check_moisture_alerts().await {
            Ok(alerts) if !alerts.is_empty() => {
                info!("Found {} moisture alerts", alerts.len());
            }
            Err(e) => {
                error!("Moisture alert check failed: {}", e);
            }
            _ => {}
        }

        match alert_system.check_strain_alerts().await {
            Ok(alerts) if !alerts.is_empty() => {
                info!("Found {} strain alerts", alerts.len());
            }
            Err(e) => {
                error!("Strain alert check failed: {}", e);
            }
            _ => {}
        }
    }
}

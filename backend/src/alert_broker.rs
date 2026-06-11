use crate::db::DbPool;
use crate::config::AlertConfig;
use crate::models::{Alert, MoistureData, StrainData};
use crate::nbiot_ingest::IngestEvent;
use crate::metrics::Metrics;
use tokio::sync::mpsc::Receiver;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, error, warn};

struct SensorReading {
    value: f64,
    time: DateTime<Utc>,
}

pub struct AlertBroker {
    pool: DbPool,
    config: AlertConfig,
    rx: Receiver<IngestEvent>,
    moisture_history: HashMap<i32, Vec<SensorReading>>,
    strain_last: HashMap<i32, SensorReading>,
}

impl AlertBroker {
    pub fn new(pool: DbPool, config: AlertConfig, rx: Receiver<IngestEvent>) -> Self {
        Self {
            pool,
            config,
            rx,
            moisture_history: HashMap::new(),
            strain_last: HashMap::new(),
        }
    }

    pub async fn run(mut self) {
        info!("Alert broker started, listening for ingest events");

        while let Some(event) = self.rx.recv().await {
            match event {
                IngestEvent::MoistureReading(data) => {
                    self.handle_moisture(data).await;
                }
                IngestEvent::StrainReading(data) => {
                    self.handle_strain(data).await;
                }
            }
        }

        warn!("Alert broker channel closed, exiting");
    }

    async fn handle_moisture(&mut self, data: MoistureData) {
        let sensor_id = data.sensor_id;
        let lacquer_ware_id = data.lacquer_ware_id;
        let threshold = self.config.moisture_drop_threshold;

        let alert_info = {
            let history = self.moisture_history.entry(sensor_id).or_default();
            history.push(SensorReading { value: data.moisture_content, time: data.time });

            if history.len() < 2 {
                None
            } else {
                while history.len() > 10 {
                    history.remove(0);
                }

                let first = &history[0];
                let last = &history[history.len() - 1];
                let time_diff_hours = (last.time - first.time).num_seconds() as f64 / 3600.0;

                if time_diff_hours >= 0.5 {
                    let drop_per_hour = (first.value - last.value) / time_diff_hours.max(0.01);
                    if drop_per_hour > threshold {
                        let severity = if drop_per_hour > 20.0 { "critical" } else { "warning" };
                        let msg = format!("含水率突降: {:.2}%/小时，超过阈值 {}%/小时", drop_per_hour, threshold);
                        history.clear();
                        Some((severity.to_string(), msg, drop_per_hour, threshold))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };

        if let Some((severity, message, drop_per_hour, threshold_val)) = alert_info {
            warn!(
                "Moisture drop alert: sensor {}, rate {:.2}%/h",
                sensor_id, drop_per_hour
            );

            match self.create_and_send_alert(
                "moisture_drop",
                &severity,
                Some(lacquer_ware_id),
                Some(sensor_id),
                message,
                Some(drop_per_hour),
                Some(threshold_val),
            ).await {
                Ok(alert) => {
                    info!("Moisture alert created: id={}", alert.id);
                }
                Err(e) => error!("Failed to create moisture alert: {}", e),
            }
        }
    }

    async fn handle_strain(&mut self, data: StrainData) {
        let sensor_id = data.sensor_id;

        if data.strain_value > self.config.strain_threshold {
            warn!(
                "Strain exceed alert: sensor {}, strain {:.2}%",
                sensor_id, data.strain_value
            );

            match self.create_and_send_alert(
                "strain_exceed",
                if data.strain_value > 8.0 { "critical" } else { "warning" },
                Some(data.lacquer_ware_id),
                Some(sensor_id),
                format!("收缩应变超标: {:.2}%，超过阈值 {}%",
                    data.strain_value, self.config.strain_threshold),
                Some(data.strain_value),
                Some(self.config.strain_threshold),
            ).await {
                Ok(alert) => {
                    info!("Strain alert created: id={}", alert.id);
                }
                Err(e) => error!("Failed to create strain alert: {}", e),
            }
        }

        self.strain_last.insert(sensor_id, SensorReading {
            value: data.strain_value,
            time: data.time,
        });
    }

    async fn create_and_send_alert(
        &self,
        alert_type: &str,
        severity: &str,
        lacquer_ware_id: Option<i32>,
        sensor_id: Option<i32>,
        message: String,
        value: Option<f64>,
        threshold: Option<f64>,
    ) -> Result<Alert, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.pool.get().await?;

        let row = client.query_one(
            r#"
            INSERT INTO alerts (alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, alert_type, severity, lacquer_ware_id, sensor_id, message, value, threshold, is_acknowledged, created_at
            "#,
            &[&alert_type, &severity, &lacquer_ware_id, &sensor_id, &message, &value, &threshold],
        ).await?;

        let alert = Alert {
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
        };

        self.send_sms_alert(&alert).await.ok();
        self.send_satellite_alert(&alert).await.ok();

        Metrics::get().alerts_triggered_total.inc();

        Ok(alert)
    }

    async fn send_sms_alert(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = format!(
            "【漆器监测告警】{} - {} - {}",
            alert.severity,
            alert.alert_type,
            alert.message.as_deref().unwrap_or("未知异常")
        );

        info!("发送短信告警到 {}: {}", self.config.alert_phone_number, message);

        if self.config.sms_api_url.is_empty() || self.config.sms_api_key.is_empty() {
            info!("SMS API not configured, skipping SMS notification");
            self.record_notification(alert.id, "sms", &self.config.alert_phone_number, "skipped", "API not configured").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.sms_api_url)
            .header("Authorization", format!("Bearer {}", self.config.sms_api_key))
            .json(&serde_json::json!({
                "phone": self.config.alert_phone_number,
                "message": message
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = if resp.status().is_success() { "sent" } else { "failed" };
                let body = resp.text().await.unwrap_or_default();
                self.record_notification(alert.id, "sms", &self.config.alert_phone_number, status, &body).await?;
            }
            Err(e) => {
                error!("SMS notification failed: {}", e);
                self.record_notification(alert.id, "sms", &self.config.alert_phone_number, "failed", &e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn send_satellite_alert(&self, alert: &Alert) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = format!(
            "LACQUER_ALERT|{}|{}|{}|{}",
            alert.id,
            alert.severity,
            alert.alert_type,
            alert.message.as_deref().unwrap_or("unknown")
        );

        info!("发送卫星通信告警到 {}: {}", self.config.satellite_recipient, message);

        if self.config.satellite_api_url.is_empty() || self.config.satellite_api_key.is_empty() {
            info!("Satellite API not configured, skipping satellite notification");
            self.record_notification(alert.id, "satellite", &self.config.satellite_recipient, "skipped", "API not configured").await?;
            return Ok(());
        }

        let client = reqwest::Client::new();
        let response = client
            .post(&self.config.satellite_api_url)
            .header("X-API-Key", &self.config.satellite_api_key)
            .json(&serde_json::json!({
                "recipient": self.config.satellite_recipient,
                "payload": message
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = if resp.status().is_success() { "sent" } else { "failed" };
                let body = resp.text().await.unwrap_or_default();
                self.record_notification(alert.id, "satellite", &self.config.satellite_recipient, status, &body).await?;
            }
            Err(e) => {
                error!("Satellite notification failed: {}", e);
                self.record_notification(alert.id, "satellite", &self.config.satellite_recipient, "failed", &e.to_string()).await?;
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
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

    pub async fn get_recent_alerts(&self, limit: i64) -> Result<Vec<Alert>, Box<dyn std::error::Error + Send + Sync>> {
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

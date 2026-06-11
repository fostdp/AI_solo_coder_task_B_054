use crate::db::DbPool;
use crate::config::NbIotConfig;
use crate::models::{NbIotDataPacket, MoistureData, StrainData};
use crate::metrics::Metrics;
use tokio::sync::mpsc::Sender;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::Type;
use chrono::{DateTime, Utc};
use tracing::{info, warn, error};

#[derive(Debug, Clone)]
pub enum IngestEvent {
    MoistureReading(MoistureData),
    StrainReading(StrainData),
}

#[derive(Clone)]
pub struct NbiotIngestService {
    pool: DbPool,
    config: NbIotConfig,
    alert_tx: Sender<IngestEvent>,
}

impl NbiotIngestService {
    pub fn new(pool: DbPool, config: NbIotConfig, alert_tx: Sender<IngestEvent>) -> Self {
        Self { pool, config, alert_tx }
    }

    pub fn config(&self) -> &NbIotConfig {
        &self.config
    }

    pub async fn ingest_single(&self, packet: &NbIotDataPacket) -> Result<(), String> {
        let client = self.pool.get().await
            .map_err(|e| {
                Metrics::get().nbiot_packets_failed.inc();
                format!("Database pool error: {}", e)
            })?;

        let sensor_row = client.query_opt(
            "SELECT id, lacquer_ware_id, sensor_type FROM sensors WHERE device_id = $1",
            &[&packet.device_id],
        ).await.map_err(|e| {
            Metrics::get().nbiot_packets_failed.inc();
            format!("Sensor lookup error: {}", e)
        })?;

        let (sensor_id, lacquer_id, sensor_type) = match sensor_row {
            Some(row) => (
                row.get::<_, i32>("id"),
                row.get::<_, Option<i32>>("lacquer_ware_id"),
                row.get::<_, String>("sensor_type"),
            ),
            None => {
                Metrics::get().nbiot_packets_failed.inc();
                return Err("Sensor not found".to_string());
            }
        };

        let lacquer_id = match lacquer_id {
            Some(id) => id,
            None => {
                Metrics::get().nbiot_packets_failed.inc();
                return Err("Sensor not assigned to any lacquer ware".to_string());
            }
        };

        match sensor_type.as_str() {
            "moisture" => {
                client.execute(
                    r#"
                    INSERT INTO moisture_data (time, sensor_id, lacquer_ware_id, moisture_content, temperature, battery_level, signal_strength)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                    &[
                        &packet.timestamp,
                        &sensor_id,
                        &lacquer_id,
                        &packet.value,
                        &packet.temperature,
                        &packet.battery_level,
                        &packet.signal_strength,
                    ],
                ).await.map_err(|e| {
                    Metrics::get().nbiot_packets_failed.inc();
                    format!("Insert error: {}", e)
                })?;

                Metrics::get().nbiot_packets_total.inc();
                Metrics::get().moisture_readings_total.inc();

                let data = MoistureData {
                    time: packet.timestamp,
                    sensor_id,
                    lacquer_ware_id: lacquer_id,
                    moisture_content: packet.value,
                    temperature: packet.temperature,
                    raw_value: None,
                    battery_level: packet.battery_level,
                    signal_strength: packet.signal_strength,
                };

                if let Err(e) = self.alert_tx.try_send(IngestEvent::MoistureReading(data)) {
                    warn!("Alert channel full, dropping event: {}", e);
                }
            }
            "strain" => {
                client.execute(
                    r#"
                    INSERT INTO strain_data (time, sensor_id, lacquer_ware_id, strain_value, temperature, battery_level, signal_strength)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                    &[
                        &packet.timestamp,
                        &sensor_id,
                        &lacquer_id,
                        &packet.value,
                        &packet.temperature,
                        &packet.battery_level,
                        &packet.signal_strength,
                    ],
                ).await.map_err(|e| {
                    Metrics::get().nbiot_packets_failed.inc();
                    format!("Insert error: {}", e)
                })?;

                Metrics::get().nbiot_packets_total.inc();
                Metrics::get().strain_readings_total.inc();

                let data = StrainData {
                    time: packet.timestamp,
                    sensor_id,
                    lacquer_ware_id: lacquer_id,
                    strain_value: packet.value,
                    temperature: packet.temperature,
                    raw_value: None,
                    battery_level: packet.battery_level,
                    signal_strength: packet.signal_strength,
                };

                if let Err(e) = self.alert_tx.try_send(IngestEvent::StrainReading(data)) {
                    warn!("Alert channel full, dropping event: {}", e);
                }
            }
            _ => {
                Metrics::get().nbiot_packets_failed.inc();
                return Err("Unknown sensor type".to_string());
            }
        }

        Ok(())
    }

    pub async fn ingest_batch(&self, packets: &[NbIotDataPacket]) -> Result<(usize, usize), String> {
        if packets.len() > self.config.max_batch_size {
            return Err(format!("Batch size exceeds limit of {}", self.config.max_batch_size));
        }

        let client = self.pool.get().await
            .map_err(|e| format!("Database pool error: {}", e))?;

        let device_ids: Vec<String> = packets.iter().map(|p| p.device_id.clone()).collect();

        let sensor_rows = client.query(
            "SELECT id, device_id, lacquer_ware_id, sensor_type FROM sensors WHERE device_id = ANY($1)",
            &[&device_ids],
        ).await.map_err(|e| format!("Sensor lookup failed: {}", e))?;

        let mut sensor_map: std::collections::HashMap<String, (i32, i32, String)> =
            std::collections::HashMap::new();
        for row in sensor_rows {
            let id: i32 = row.get("id");
            let device_id: String = row.get("device_id");
            let lacquer_ware_id: Option<i32> = row.get("lacquer_ware_id");
            let sensor_type: String = row.get("sensor_type");
            if let Some(lid) = lacquer_ware_id {
                sensor_map.insert(device_id, (id, lid, sensor_type));
            }
        }

        let mut moisture_rows: Vec<(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)> = Vec::new();
        let mut strain_rows: Vec<(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)> = Vec::new();
        let mut fail_count = 0usize;

        let mut moisture_events = Vec::new();
        let mut strain_events = Vec::new();

        for packet in packets {
            match sensor_map.get(&packet.device_id) {
                Some((sensor_id, lacquer_id, sensor_type)) => {
                    let row = (
                        packet.timestamp,
                        *sensor_id,
                        *lacquer_id,
                        packet.value,
                        packet.temperature,
                        packet.battery_level,
                        packet.signal_strength,
                    );
                    if sensor_type == "moisture" {
                        moisture_rows.push(row);
                        moisture_events.push(MoistureData {
                            time: packet.timestamp,
                            sensor_id: *sensor_id,
                            lacquer_ware_id: *lacquer_id,
                            moisture_content: packet.value,
                            temperature: packet.temperature,
                            raw_value: None,
                            battery_level: packet.battery_level,
                            signal_strength: packet.signal_strength,
                        });
                    } else if sensor_type == "strain" {
                        strain_rows.push(row);
                        strain_events.push(StrainData {
                            time: packet.timestamp,
                            sensor_id: *sensor_id,
                            lacquer_ware_id: *lacquer_id,
                            strain_value: packet.value,
                            temperature: packet.temperature,
                            raw_value: None,
                            battery_level: packet.battery_level,
                            signal_strength: packet.signal_strength,
                        });
                    } else {
                        fail_count += 1;
                    }
                }
                None => { fail_count += 1; }
            }
        }

        let mut success_count: usize = 0;

        if !moisture_rows.is_empty() {
            let inserted = batch_copy_insert_moisture(&client, &moisture_rows).await;
            success_count += inserted;
            if inserted > 0 {
                for evt in moisture_events.iter().take(inserted) {
                    if let Err(e) = self.alert_tx.try_send(IngestEvent::MoistureReading(evt.clone())) {
                        warn!("Alert channel full, dropping moisture event: {}", e);
                    }
                }
            }
        }

        if !strain_rows.is_empty() {
            let inserted = batch_copy_insert_strain(&client, &strain_rows).await;
            success_count += inserted;
            if inserted > 0 {
                for evt in strain_events.iter().take(inserted) {
                    if let Err(e) = self.alert_tx.try_send(IngestEvent::StrainReading(evt.clone())) {
                        warn!("Alert channel full, dropping strain event: {}", e);
                    }
                }
            }
        }

        info!("NB-IoT batch ingest: {}/{} succeeded, {} failed", success_count, success_count + fail_count, fail_count);

        let metrics = Metrics::get();
        metrics.nbiot_packets_total.inc_by(success_count as u64);
        metrics.nbiot_packets_failed.inc_by(fail_count as u64);
        metrics.moisture_readings_total.inc_by(moisture_rows.len() as u64);
        metrics.strain_readings_total.inc_by(strain_rows.len() as u64);

        Ok((success_count, fail_count))
    }
}

async fn batch_copy_insert_moisture(
    client: &deadpool_postgres::Object,
    rows: &[(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)],
) -> usize {
    let copy_stmt = match client.prepare(
        "COPY moisture_data (time, sensor_id, lacquer_ware_id, moisture_content, temperature, battery_level, signal_strength) FROM STDIN WITH (FORMAT binary)"
    ).await {
        Ok(s) => s,
        Err(e) => {
            error!("COPY moisture_data prepare failed, falling back to INSERT: {}", e);
            return batch_fallback_insert_moisture(client, rows).await;
        }
    };

    let types = [Type::TIMESTAMPTZ, Type::INT4, Type::INT4, Type::FLOAT8, Type::FLOAT8, Type::FLOAT8, Type::FLOAT8];
    let sink = match client.copy_in(&copy_stmt).await {
        Ok(s) => s,
        Err(e) => {
            error!("COPY moisture_data init failed, falling back: {}", e);
            return batch_fallback_insert_moisture(client, rows).await;
        }
    };

    let writer = BinaryCopyInWriter::new(sink, &types);
    let mut writer = std::pin::pin!(writer);

    for row in rows {
        if writer.as_mut().write(&[
            &row.0 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.1 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.2 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.3 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.4 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.5 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.6 as &(dyn tokio_postgres::types::ToSql + Sync),
        ]).await.is_err() {
            error!("COPY moisture_data write failed");
            return batch_fallback_insert_moisture(client, rows).await;
        }
    }

    match writer.finish().await {
        Ok(_) => {
            info!("COPY moisture_data: {} rows via binary COPY", rows.len());
            rows.len()
        }
        Err(e) => {
            error!("COPY moisture_data finish failed: {}", e);
            batch_fallback_insert_moisture(client, rows).await
        }
    }
}

async fn batch_fallback_insert_moisture(
    client: &deadpool_postgres::Object,
    rows: &[(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)],
) -> usize {
    let mut count = 0usize;
    for row in rows {
        match client.execute(
            "INSERT INTO moisture_data (time, sensor_id, lacquer_ware_id, moisture_content, temperature, battery_level, signal_strength) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&row.0, &row.1, &row.2, &row.3, &row.4, &row.5, &row.6],
        ).await {
            Ok(_) => count += 1,
            Err(e) => warn!("Fallback INSERT moisture failed: {}", e),
        }
    }
    count
}

async fn batch_copy_insert_strain(
    client: &deadpool_postgres::Object,
    rows: &[(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)],
) -> usize {
    let copy_stmt = match client.prepare(
        "COPY strain_data (time, sensor_id, lacquer_ware_id, strain_value, temperature, battery_level, signal_strength) FROM STDIN WITH (FORMAT binary)"
    ).await {
        Ok(s) => s,
        Err(e) => {
            error!("COPY strain_data prepare failed, falling back to INSERT: {}", e);
            return batch_fallback_insert_strain(client, rows).await;
        }
    };

    let types = [Type::TIMESTAMPTZ, Type::INT4, Type::INT4, Type::FLOAT8, Type::FLOAT8, Type::FLOAT8, Type::FLOAT8];
    let sink = match client.copy_in(&copy_stmt).await {
        Ok(s) => s,
        Err(e) => {
            error!("COPY strain_data init failed, falling back: {}", e);
            return batch_fallback_insert_strain(client, rows).await;
        }
    };

    let writer = BinaryCopyInWriter::new(sink, &types);
    let mut writer = std::pin::pin!(writer);

    for row in rows {
        if writer.as_mut().write(&[
            &row.0 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.1 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.2 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.3 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.4 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.5 as &(dyn tokio_postgres::types::ToSql + Sync),
            &row.6 as &(dyn tokio_postgres::types::ToSql + Sync),
        ]).await.is_err() {
            error!("COPY strain_data write failed");
            return batch_fallback_insert_strain(client, rows).await;
        }
    }

    match writer.finish().await {
        Ok(_) => {
            info!("COPY strain_data: {} rows via binary COPY", rows.len());
            rows.len()
        }
        Err(e) => {
            error!("COPY strain_data finish failed: {}", e);
            batch_fallback_insert_strain(client, rows).await
        }
    }
}

async fn batch_fallback_insert_strain(
    client: &deadpool_postgres::Object,
    rows: &[(DateTime<Utc>, i32, i32, f64, Option<f64>, Option<f64>, Option<f64>)],
) -> usize {
    let mut count = 0usize;
    for row in rows {
        match client.execute(
            "INSERT INTO strain_data (time, sensor_id, lacquer_ware_id, strain_value, temperature, battery_level, signal_strength) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&row.0, &row.1, &row.2, &row.3, &row.4, &row.5, &row.6],
        ).await {
            Ok(_) => count += 1,
            Err(e) => warn!("Fallback INSERT strain failed: {}", e),
        }
    }
    count
}

use prometheus::{
    register_int_counter, register_int_gauge, register_histogram,
    IntCounter, IntGauge, Histogram, HistogramOpts, Opts,
};
use std::sync::OnceLock;

pub struct Metrics {
    pub http_requests_total: IntCounter,
    pub http_request_duration_seconds: Histogram,
    pub http_active_requests: IntGauge,
    pub nbiot_packets_total: IntCounter,
    pub nbiot_packets_failed: IntCounter,
    pub alerts_triggered_total: IntCounter,
    pub db_connections_active: IntGauge,
    pub db_connections_idle: IntGauge,
    pub moisture_readings_total: IntCounter,
    pub strain_readings_total: IntCounter,
}

static METRICS: OnceLock<Metrics> = OnceLock::new();

impl Metrics {
    pub fn init() -> &'static Self {
        METRICS.get_or_init(|| {
            let http_requests_total = register_int_counter!(
                Opts::new("http_requests_total", "Total number of HTTP requests")
            ).expect("Failed to register http_requests_total");

            let http_request_duration_seconds = register_histogram!(
                HistogramOpts::new(
                    "http_request_duration_seconds",
                    "HTTP request duration in seconds"
                )
                .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
            ).expect("Failed to register http_request_duration_seconds");

            let http_active_requests = register_int_gauge!(
                Opts::new("http_active_requests", "Number of active HTTP requests")
            ).expect("Failed to register http_active_requests");

            let nbiot_packets_total = register_int_counter!(
                Opts::new("nbiot_packets_total", "Total NB-IoT packets received")
            ).expect("Failed to register nbiot_packets_total");

            let nbiot_packets_failed = register_int_counter!(
                Opts::new("nbiot_packets_failed", "Failed NB-IoT packets")
            ).expect("Failed to register nbiot_packets_failed");

            let alerts_triggered_total = register_int_counter!(
                Opts::new("alerts_triggered_total", "Total alerts triggered")
            ).expect("Failed to register alerts_triggered_total");

            let db_connections_active = register_int_gauge!(
                Opts::new("db_connections_active", "Active database connections")
            ).expect("Failed to register db_connections_active");

            let db_connections_idle = register_int_gauge!(
                Opts::new("db_connections_idle", "Idle database connections")
            ).expect("Failed to register db_connections_idle");

            let moisture_readings_total = register_int_counter!(
                Opts::new("moisture_readings_total", "Total moisture readings processed")
            ).expect("Failed to register moisture_readings_total");

            let strain_readings_total = register_int_counter!(
                Opts::new("strain_readings_total", "Total strain readings processed")
            ).expect("Failed to register strain_readings_total");

            Self {
                http_requests_total,
                http_request_duration_seconds,
                http_active_requests,
                nbiot_packets_total,
                nbiot_packets_failed,
                alerts_triggered_total,
                db_connections_active,
                db_connections_idle,
                moisture_readings_total,
                strain_readings_total,
            }
        })
    }

    pub fn get() -> &'static Self {
        METRICS.get().expect("Metrics not initialized")
    }

    pub fn gather() -> Vec<prometheus::proto::MetricFamily> {
        prometheus::gather()
    }
}

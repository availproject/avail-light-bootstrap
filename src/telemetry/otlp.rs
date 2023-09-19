use std::{sync::RwLock, time::Duration};

use anyhow::{Error, Ok, Result};
use opentelemetry_api::{global, metrics::Meter, KeyValue};
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};

pub struct Metrics {
    pub meter: Meter,
    pub peer_id: String,
    pub multiaddress: RwLock<String>,
    pub ip: RwLock<String>,
    pub role: String,
}

impl Metrics {
    fn attributes(&self) -> [KeyValue; 6] {
        [
            KeyValue::new("job", "avail_light_bootstrap"),
            KeyValue::new("version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("role", self.role.clone()),
            KeyValue::new("peerID", self.multiaddress.read().unwrap().clone()),
            KeyValue::new("multiaddress", self.multiaddress.read().unwrap().clone()),
            KeyValue::new("ip", self.ip.read().unwrap().clone()),
        ]
    }

    fn record_u64(&self, name: &'static str, value: u64) -> Result<()> {
        let instrument = self.meter.u64_observable_gauge(name).try_init()?;
        let attributes = self.attributes();
        self.meter
            .register_callback(&[instrument.as_any()], move |observer| {
                observer.observe_u64(&instrument, value, &attributes)
            })?;
        Ok(())
    }
}

impl super::Metrics for Metrics {
    fn record(&self, value: super::MetricValue) -> Result<()> {
        match value {
            super::MetricValue::ActivePeers(num) => {
                self.record_u64("active_peers", num.into())?;
            }
        }
        Ok(())
    }
}

pub fn initialize(endpoint: String, peer_id: String, role: String) -> Result<Metrics, Error> {
    let export_config = ExportConfig {
        endpoint,
        timeout: Duration::from_secs(10),
        protocol: Protocol::Grpc,
    };
    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(opentelemetry_sdk::runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config),
        )
        .with_period(Duration::from_secs(10))
        .with_timeout(Duration::from_secs(15))
        .build()?;

    global::set_meter_provider(provider);
    let meter = global::meter("avail_light_bootstrap");

    Ok(Metrics {
        meter,
        peer_id,
        multiaddress: RwLock::new("".to_string()),
        ip: RwLock::new("".to_string()),
        role,
    })
}

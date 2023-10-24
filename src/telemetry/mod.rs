pub mod otlp;

use anyhow::Result;
use async_trait::async_trait;

pub enum MetricValue {
    KadRoutingPeerNum(usize),
    HealthCheck(),
}

#[async_trait]
pub trait Metrics {
    async fn record(&self, value: MetricValue) -> Result<()>;
    async fn set_multiaddress(&self, multiaddrs: String);
    async fn set_ip(&self, ip: String);
}

use anyhow::Result;

pub mod otlp;

pub enum MetricValue {
    ActivePeers(u32),
}

pub trait Metrics {
    fn record(&self, value: MetricValue) -> Result<()>;
}

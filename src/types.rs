use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, net::SocketAddr, str::FromStr, time::Duration};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SecretKey {
    Seed { seed: String },
    Key { key: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Bootstrap HTTP server host name (default: 127.0.0.1).
    pub http_server_host: String,
    /// Bootstrap HTTP server port (default: 7700).
    pub http_server_port: u16,
    /// Log level. See `<https://docs.rs/log/0.4.17/log/enum.LevelFilter.html>` for possible log level values. (default: `INFO`)
    pub log_level: String,
    /// Set to display structured logs in JSON format. Otherwise, plain text format is used. (default: false)
    pub log_format_json: bool,
    /// Sets the listening P2P network service port. (default: 39000)
    pub port: u16,
    /// Sets the amount of time to keep connections alive when they're idle. (default: 30s).
    /// NOTE: libp2p default value is 10s, but because of Avail block time of 20s the value has been increased
    pub connection_idle_timeout: u64,
    /// Sets application-specific version of the protocol family used by the peer. (default: "/avail_kad/id/1.0.0")
    pub identify_protocol: String,
    /// Sets agent version that is sent to peers in the network. (default: "avail-light-client/rust-client")
    pub identify_agent: String,
    /// Configures AutoNAT behaviour to reject probes as a server for clients that are observed at a non-global ip address (default: false)
    pub autonat_only_global_ips: bool,
    /// Sets the timeout for a single Kademlia query. (default: 60s).
    pub kad_query_timeout: u32,
    /// Defines a period of time in which periodic bootstraps will be repeated. (default: 300s)
    pub bootstrap_period: u64,
    /// OpenTelemetry Collector endpoint (default: http://otelcollector.avail.tools:4317)
    pub ot_collector_endpoint: String,
    /// Defines a period of time in which periodic metric network dump events will be repeated. (default: 15s)
    pub metrics_network_dump_interval: u64,
    /// Secret key used to generate keypair. Can be either set to `seed` or to `key`. (default: seed="1")
    /// If set to seed, keypair will be generated from that seed.
    /// If set to key, a valid ed25519 private key must be provided, else the client will fail
    /// If `secret_key` is not set, random seed will be used.
    /// Default bootstrap peerID is 12D3KooWStAKPADXqJ7cngPYXd2mSANpdgh1xQ34aouufHA2xShz
    pub secret_key: Option<SecretKey>,
}

pub struct LibP2PConfig {
    pub port: u16,
    pub autonat_only_global_ips: bool,
    pub identify_agent_version: String,
    pub identify_protocol_version: String,
    pub kademlia: KademliaConfig,
    pub secret_key: Option<SecretKey>,
    pub bootstrap_interval: Duration,
    pub connection_idle_timeout: Duration,
}

impl From<&RuntimeConfig> for LibP2PConfig {
    fn from(rtcfg: &RuntimeConfig) -> Self {
        Self {
            port: rtcfg.port,
            autonat_only_global_ips: rtcfg.autonat_only_global_ips,
            identify_agent_version: rtcfg.identify_agent.clone(),
            identify_protocol_version: rtcfg.identify_protocol.clone(),
            kademlia: rtcfg.into(),
            secret_key: rtcfg.secret_key.clone(),
            bootstrap_interval: Duration::from_secs(rtcfg.bootstrap_period),
            connection_idle_timeout: Duration::from_secs(rtcfg.connection_idle_timeout),
        }
    }
}

/// Kademlia configuration (see [RuntimeConfig] for details)
pub struct KademliaConfig {
    pub query_timeout: Duration,
}

impl From<&RuntimeConfig> for KademliaConfig {
    fn from(val: &RuntimeConfig) -> Self {
        KademliaConfig {
            query_timeout: Duration::from_secs(val.kad_query_timeout.into()),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            http_server_host: "127.0.0.1".to_owned(),
            http_server_port: 7700,
            log_level: "INFO".to_string(),
            log_format_json: false,
            secret_key: Some(SecretKey::Seed {
                seed: "1".to_string(),
            }),
            port: 39000,
            autonat_only_global_ips: false,
            identify_protocol: "/avail_kad/id/1.0.0".to_string(),
            identify_agent: "avail-light-client/rust-client".to_string(),
            connection_idle_timeout: 30,
            kad_query_timeout: 60,
            bootstrap_period: 300,
            ot_collector_endpoint: "http://otelcollector.avail.tools:4317".to_string(),
            metrics_network_dump_interval: 15,
        }
    }
}

pub struct Addr {
    pub host: String,
    pub port: u16,
}

impl From<&RuntimeConfig> for Addr {
    fn from(value: &RuntimeConfig) -> Self {
        Addr {
            host: value.http_server_host.clone(),
            port: value.http_server_port,
        }
    }
}

impl TryInto<SocketAddr> for Addr {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<SocketAddr, Self::Error> {
        SocketAddr::from_str(&format!("{self}")).context("Unable to parse host and port")
    }
}

impl Display for Addr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

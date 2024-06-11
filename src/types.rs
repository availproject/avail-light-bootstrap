use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    net::SocketAddr,
    str::FromStr,
    time::Duration,
};

const MINIMUM_SUPPORTED_VERSION: &str = "1.9.2";
pub const IDENTITY_PROTOCOL: &str = "/avail_kad/id/1.0.0";
pub const IDENTITY_AGENT_BASE: &str = "avail-light-client";
pub const IDENTITY_AGENT_CLIENT_TYPE: &str = "rust-client";

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
    /// Enable WebSocket transport over TCP
    pub ws_transport_enable: bool,
    /// Sets the amount of time to keep connections alive when they're idle. (default: 30s).
    /// NOTE: libp2p default value is 10s, but because of Avail block time of 20s the value has been increased
    pub connection_idle_timeout: u64,
    /// Autonat server config - max total dial requests (Default: 30).
    pub autonat_throttle_clients_global_max: usize,
    /// Autonat server config - max dial requests for a single peer (Default: 3).
    pub autonat_throttle_clients_peer_max: usize,
    /// Autonat server config - period for throttling clients requests (Default 1s).
    pub autonat_throttle_clients_period: u32,
    /// Autonat server config - configures AutoNAT behaviour to reject probes as a server for clients that are observed at a non-global ip address (default: true)
    pub autonat_only_global_ips: bool,
    /// Sets the timeout for a single Kademlia query. (default: 60s).
    pub kad_query_timeout: u32,
    /// Defines a period of time in which periodic bootstraps will be repeated. (default: 300s)
    pub bootstrap_period: u64,
    /// OpenTelemetry Collector endpoint (default: http://127.0.0.1:4317)
    pub ot_collector_endpoint: String,
    /// Defines a period of time in which periodic metric network dump events will be repeated. (default: 15s)
    pub metrics_network_dump_interval: u64,
    /// Secret key used to generate keypair. Can be either set to `seed` or to `key`. (default: seed="1")
    /// If set to seed, keypair will be generated from that seed.
    /// If set to key, a valid ed25519 private key must be provided, else the client will fail
    /// If `secret_key` is not set, random seed will be used.
    /// Default bootstrap peerID is 12D3KooWStAKPADXqJ7cngPYXd2mSANpdgh1xQ34aouufHA2xShz
    pub secret_key: Option<SecretKey>,
    pub origin: String,
    /// Genesis hash of the network to be connected to. Set to a string beginning with "DEV" to connect to any network.
    pub genesis_hash: String,
}

pub struct LibP2PConfig {
    pub port: u16,
    pub autonat: AutonatConfig,
    pub identify: IdentifyConfig,
    pub kademlia: KademliaConfig,
    pub secret_key: Option<SecretKey>,
    pub bootstrap_interval: Duration,
    pub connection_idle_timeout: Duration,
}

impl From<&RuntimeConfig> for LibP2PConfig {
    fn from(rtcfg: &RuntimeConfig) -> Self {
        Self {
            port: rtcfg.port,
            autonat: rtcfg.into(),
            identify: rtcfg.into(),
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

pub struct AutonatConfig {
    pub throttle_clients_global_max: usize,
    pub throttle_clients_peer_max: usize,
    pub throttle_clients_period: Duration,
    pub only_global_ips: bool,
}

impl From<&RuntimeConfig> for AutonatConfig {
    fn from(val: &RuntimeConfig) -> Self {
        AutonatConfig {
            throttle_clients_global_max: val.autonat_throttle_clients_global_max,
            throttle_clients_peer_max: val.autonat_throttle_clients_peer_max,
            throttle_clients_period: Duration::from_secs(
                val.autonat_throttle_clients_period.into(),
            ),
            only_global_ips: val.autonat_only_global_ips,
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
            ws_transport_enable: false,
            autonat_throttle_clients_global_max: 120,
            autonat_throttle_clients_peer_max: 4,
            autonat_throttle_clients_period: 1,
            autonat_only_global_ips: true,
            connection_idle_timeout: 30,
            kad_query_timeout: 60,
            bootstrap_period: 300,
            ot_collector_endpoint: "http://127.0.0.1:4317".to_string(),
            metrics_network_dump_interval: 15,
            origin: "external".to_string(),
            genesis_hash: "DEV".to_owned(),
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

pub struct IdentifyConfig {
    pub agent_version: AgentVersion,
    /// Contains Avail genesis hash
    pub protocol_version: String,
    pub minimum_supported_version: String,
}

impl IdentifyConfig {
    pub fn is_supported(&self, version: &str) -> bool {
        self.minimum_supported_version
            .split('.')
            .map(|s| s.parse().unwrap_or(0))
            .zip(version.split('.').map(|s| s.parse().unwrap_or(0)))
            .find(|(old, new)| old != new)
            .map_or(true, |(old, new)| new > old)
    }
}

pub struct AgentVersion {
    pub base_version: String,
    pub client_type: String,
    // Kademlia client or server mode
    pub kademlia_mode: String,
}

impl fmt::Display for AgentVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.base_version, self.client_type, self.kademlia_mode
        )
    }
}

impl FromStr for AgentVersion {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 3 {
            return Err("Failed to parse agent version".to_owned());
        }

        Ok(AgentVersion {
            base_version: parts[0].to_string(),
            client_type: parts[1].to_string(),
            kademlia_mode: parts[2].to_string(),
        })
    }
}

impl From<&RuntimeConfig> for IdentifyConfig {
    fn from(val: &RuntimeConfig) -> Self {
        let mut genhash_short = val.genesis_hash.trim_start_matches("0x").to_string();
        genhash_short.truncate(6);

        let agent_version = AgentVersion {
            base_version: IDENTITY_AGENT_BASE.to_string(),
            client_type: IDENTITY_AGENT_CLIENT_TYPE.to_string(),
            // Bootstrap should only be in server mode
            kademlia_mode: "server".to_string(),
        };

        Self {
            agent_version,
            protocol_version: format!(
                "{id}-{gen_hash}",
                id = IDENTITY_PROTOCOL,
                gen_hash = genhash_short
            ),
            minimum_supported_version: MINIMUM_SUPPORTED_VERSION.to_string(),
        }
    }
}

pub fn network_name(genesis_hash: &str) -> String {
    let network = match genesis_hash {
        "9d5ea6a5d7631e13028b684a1a0078e3970caa78bd677eaecaf2160304f174fb" => "hex".to_string(),
        "d3d2f3a3495dc597434a99d7d449ebad6616db45e4e4f178f31cc6fa14378b70" => "turing".to_string(),
        "DEV" => "local".to_string(),
        _ => "other".to_string(),
    };

    format!("{}:{}", network, &genesis_hash[..6])
}

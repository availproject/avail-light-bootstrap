use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum SecretKey {
    Seed { seed: String },
    Key { key: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Log level. See `<https://docs.rs/log/0.4.17/log/enum.LevelFilter.html>` for possible log level values. (default: `INFO`)
    pub log_level: String,
    /// Set to display structured logs in JSON format. Otherwise, plain text format is used. (default: false)
    pub log_format_json: bool,
    /// Secret key for used to generate keypair. Can be either set to `seed` or to `key`.
    /// If set to seed, keypair will be generated from that seed.
    /// If set to key, a valid ed25519 private key must be provided, else the client will fail
    /// If `secret_key` is not set, random seed will be used.
    pub secret_key: Option<SecretKey>,
    /// Sets the listening P2P network service port. (default: 37000)
    pub p2p_port: u16,
    /// Sets application-specific version of the protocol family used by the peer. (default: "/avail_kad/id/1.0.0")
    pub identify_protocol: String,
    /// Sets agent version that is sent to peers in the network. (default: "avail-light-client/rust-client")
    pub identify_agent: String,
    /// Configures AutoNAT behaviour to reject probes as a server for clients that are observed at a non-global ip address (default: false)
    pub autonat_only_global_ips: bool,
    /// Sets the amount of time to keep connections alive when they're idle. (default: 30s).
    /// NOTE: libp2p default value is 10s, but because of Avail block time of 20s the value has been increased
    pub kad_connection_idle_timeout: u32,
    /// Sets the timeout for a single Kademlia query. (default: 60s).
    pub kad_query_timeout: u32,
}

pub struct LibP2PConfig {
    pub port: u16,
    pub autonat_only_global_ips: bool,
    pub identify_agent_version: String,
    pub identify_protocol_version: String,
    pub kademlia: KademliaConfig,
}

impl From<&RuntimeConfig> for LibP2PConfig {
    fn from(rtcfg: &RuntimeConfig) -> Self {
        Self {
            port: rtcfg.p2p_port,
            autonat_only_global_ips: rtcfg.autonat_only_global_ips,
            identify_agent_version: rtcfg.identify_agent.clone(),
            identify_protocol_version: rtcfg.identify_protocol.clone(),
            kademlia: rtcfg.into(),
        }
    }
}

/// Kademlia configuration (see [RuntimeConfig] for details)
pub struct KademliaConfig {
    pub connection_idle_timeout: Duration,
    pub query_timeout: Duration,
}

impl From<&RuntimeConfig> for KademliaConfig {
    fn from(val: &RuntimeConfig) -> Self {
        KademliaConfig {
            connection_idle_timeout: Duration::from_secs(val.kad_connection_idle_timeout.into()),
            query_timeout: Duration::from_secs(val.kad_query_timeout.into()),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            log_level: "INFO".to_string(),
            log_format_json: false,
            secret_key: None,
            p2p_port: 37000,
            autonat_only_global_ips: false,
            identify_protocol: "/avail_kad/id/1.0.0".to_string(),
            identify_agent: "avail-light-client/rust-client".to_string(),
            kad_connection_idle_timeout: 30,
            kad_query_timeout: 60,
        }
    }
}

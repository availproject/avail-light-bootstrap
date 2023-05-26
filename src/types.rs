use std::{num::NonZeroUsize, time::Duration};

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
    /// Secret key for libp2p keypair. Can be either set to `seed` or to `key`.
    /// If set to seed, keypair will be generated from that seed.
    /// If set to key, a valid ed25519 private key must be provided, else the client will fail
    /// If `secret_key` is not set, random seed will be used.
    pub secret_key: Option<SecretKey>,
    /// Sets Libp2p service port. (default: 37000)
    pub libp2p_port: u16,
    /// Sets libp2p application-specific version of the protocol family used by the peer. (default: "/avail_kad/id/1.0.0")
    pub libp2p_identify_protocol: String,
    /// Sets libp2p agent version that is sent to peers. (default: "avail-light-client/rust-client")
    pub libp2p_identify_agent: String,
    /// Configures LibP2P TCP port reuse for local sockets, which implies reuse of listening ports for outgoing connections to enhance NAT traversal capabilities (default: false)
    pub libp2p_tcp_port_reuse: bool,
    /// Configures LibP2P AutoNAT behaviour to reject probes as a server for clients that are observed at a non-global ip address (default: false)
    pub libp2p_autonat_only_global_ips: bool,

    /// Kademlia configuration - WARNING: Changing the default values might cause the peer to suffer poor performance!
    /// Default Kademlia config values have been copied from rust-libp2p Kademila defaults
    ///
    /// Time-to-live for DHT entries in seconds (default: 24h).
    /// Default value is set for light clients. Due to the heavy duty nature of the fat clients, it is recommended to be set far bellow this
    /// value - not greater than 1hr.
    /// Record TTL, publication and replication intervals are co-dependent, meaning that TTL >> publication_interval >> replication_interval.
    pub kad_record_ttl: u64,
    /// Sets the (re-)publication interval of stored records in seconds. (default: 12h).
    /// Default value is set for light clients. Fat client value needs to be inferred from the TTL value.
    /// This interval should be significantly shorter than the record TTL, to ensure records do not expire prematurely.
    pub publication_interval: u32,
    /// Sets the (re-)replication interval for stored records in seconds. (default: 3h).
    /// Default value is set for light clients. Fat client value needs to be inferred from the TTL and publication interval values.
    /// This interval should be significantly shorter than the publication interval, to ensure persistence between re-publications.
    pub replication_interval: u32,
    /// The replication factor determines to how many closest peers a record is replicated. (default: 20).
    pub replication_factor: u16,
    /// Sets the amount of time to keep connections alive when they're idle. (default: 30s).
    /// NOTE: libp2p default value is 10s, but because of Avail block time of 20s the value has been increased
    pub connection_idle_timeout: u32,
    /// Sets the timeout for a single Kademlia query. (default: 60s).
    pub query_timeout: u32,
    /// Sets the allowed level of parallelism for iterative Kademlia queries. (default: 3).
    pub query_parallelism: u16,
    /// Sets the Kademlia caching strategy to use for successful lookups. (default: 1).
    /// If set to 0, caching is disabled.
    pub caching_max_peers: u16,
    /// Require iterative queries to use disjoint paths for increased resiliency in the presence of potentially adversarial nodes. (default: false).
    pub disjoint_query_paths: bool,
    /// The maximum number of records. (default: 2400000).
    /// The default value has been calculated to sustain ~1hr worth of cells, in case of blocks with max sizes being produces in 20s block time for fat clients
    /// (256x512) * 3 * 60
    pub max_kad_record_number: u64,
    /// The maximum size of record values, in bytes. (default: 8192).
    pub max_kad_record_size: u64,
    /// The maximum number of provider records for which the local node is the provider. (default: 1024).
    pub max_kad_provided_keys: u64,
}

pub struct LibP2PConfig {
    pub port: u16,
    pub tcp_port_reuse: bool,
    pub autonat_only_global_ips: bool,
    pub identify_agent_version: String,
    pub identify_protocol_version: String,
    pub kademlia: KademliaConfig,
}

impl From<&RuntimeConfig> for LibP2PConfig {
    fn from(rtcfg: &RuntimeConfig) -> Self {
        Self {
            port: rtcfg.libp2p_port,
            tcp_port_reuse: rtcfg.libp2p_tcp_port_reuse,
            autonat_only_global_ips: rtcfg.libp2p_autonat_only_global_ips,
            identify_agent_version: rtcfg.libp2p_identify_agent.clone(),
            identify_protocol_version: rtcfg.libp2p_identify_protocol.clone(),
            kademlia: rtcfg.into(),
        }
    }
}

/// Kademlia configuration (see [RuntimeConfig] for details)
pub struct KademliaConfig {
    pub record_ttl: u64,
    pub record_replication_factor: NonZeroUsize,
    pub record_replication_interval: Option<Duration>,
    pub publication_interval: Option<Duration>,
    pub connection_idle_timeout: Duration,
    pub query_timeout: Duration,
    pub query_parallelism: NonZeroUsize,
    pub caching_max_peers: u16,
    pub disjoint_query_paths: bool,
    pub max_kad_record_number: usize,
    pub max_kad_record_size: usize,
    pub max_kad_provided_keys: usize,
}

impl From<&RuntimeConfig> for KademliaConfig {
    fn from(val: &RuntimeConfig) -> Self {
        KademliaConfig {
            record_ttl: val.kad_record_ttl,
            record_replication_factor: std::num::NonZeroUsize::new(val.replication_factor as usize)
                .expect("Invalid replication factor"),
            record_replication_interval: Some(Duration::from_secs(val.replication_interval.into())),
            publication_interval: Some(Duration::from_secs(val.publication_interval.into())),
            connection_idle_timeout: Duration::from_secs(val.connection_idle_timeout.into()),
            query_timeout: Duration::from_secs(val.query_timeout.into()),
            query_parallelism: std::num::NonZeroUsize::new(val.query_parallelism as usize)
                .expect("Invalid query parallelism value"),
            caching_max_peers: val.caching_max_peers,
            disjoint_query_paths: val.disjoint_query_paths,
            max_kad_record_number: val.max_kad_record_number as usize,
            max_kad_record_size: val.max_kad_record_size as usize,
            max_kad_provided_keys: val.max_kad_provided_keys as usize,
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            log_level: "INFO".to_string(),
            log_format_json: false,
            secret_key: None,
            libp2p_port: 37000,
            libp2p_tcp_port_reuse: false,
            libp2p_autonat_only_global_ips: false,
            libp2p_identify_protocol: "/avail_kad/id/1.0.0".to_string(),
            libp2p_identify_agent: "avail-light-client/rust-client".to_string(),
            kad_record_ttl: 24 * 60 * 60,
            replication_factor: 20,
            publication_interval: 12 * 60 * 60,
            replication_interval: 3 * 60 * 60,
            connection_idle_timeout: 30,
            query_timeout: 60,
            query_parallelism: 3,
            caching_max_peers: 1,
            disjoint_query_paths: false,
            max_kad_record_number: 2400000,
            max_kad_record_size: 8192,
            max_kad_provided_keys: 1024,
        }
    }
}

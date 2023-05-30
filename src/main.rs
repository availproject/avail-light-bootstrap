#![doc = include_str!("../README.md")]

mod types;

use anyhow::{Context, Result};
use clap::Parser;
use libp2p::{
    autonat,
    core::{muxing::StreamMuxerBox, transport::Boxed},
    futures::StreamExt,
    identify::{self, Event as IdentifyEvent, Info},
    identity::Keypair,
    kad::{
        self,
        store::{MemoryStore, MemoryStoreConfig},
        Kademlia, KademliaCaching, KademliaConfig,
    },
    multiaddr::Protocol,
    ping::{self, Config as PingConfig},
    quic::{tokio::Transport as QuicTransport, Config as QuicConfig},
    swarm::{NetworkBehaviour, SwarmBuilder, SwarmEvent},
    Multiaddr, PeerId, Swarm, Transport,
};
use multihash::{self, Hasher};
use std::net::Ipv4Addr;
use tracing::{debug, error, info, metadata::ParseLevelError, warn, Level};
use tracing_subscriber::{
    fmt::format::{self, DefaultFields, Format, Full, Json},
    FmtSubscriber,
};
use types::{LibP2PConfig, RuntimeConfig, SecretKey};

#[derive(Debug, Parser)]
#[clap(name = "Avail Relay Server")]
struct CliOpts {
    #[clap(
        long,
        short = 'C',
        default_value = "config.yaml",
        help = "yaml configuration file"
    )]
    config: String,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    kademlia: kad::Kademlia<MemoryStore>,
    identify: identify::Behaviour,
    auto_nat: autonat::Behaviour,
    ping: ping::Behaviour,
}

fn parse_log_lvl(log_lvl: &str, default: Level) -> (Level, Option<ParseLevelError>) {
    log_lvl
        .to_uppercase()
        .parse::<Level>()
        .map(|lvl| (lvl, None))
        .unwrap_or_else(|err| (default, Some(err)))
}

fn json_subscriber(log_lvl: Level) -> FmtSubscriber<DefaultFields, Format<Json>> {
    FmtSubscriber::builder()
        .with_max_level(log_lvl)
        .event_format(format::json())
        .finish()
}

fn default_subscriber(log_lvl: Level) -> FmtSubscriber<DefaultFields, Format<Full>> {
    FmtSubscriber::builder()
        .with_max_level(log_lvl)
        .with_span_events(format::FmtSpan::CLOSE)
        .finish()
}

fn generate_id_keys(secret_key: Option<SecretKey>) -> Result<Keypair> {
    let id_keys = match secret_key {
        // if seed was provided, then generate secret key from that seed
        Some(SecretKey::Seed { seed }) => {
            let seed_digest = multihash::Sha3_256::digest(seed.as_bytes());
            Keypair::ed25519_from_bytes(seed_digest)
                .context("could not generate keypair from seed")?
        }
        // import provided secret key
        Some(SecretKey::Key { key }) => {
            let mut decoded_key = [0u8; 32];
            hex::decode_to_slice(key.into_bytes(), &mut decoded_key)
                .context("could not decode secret key from config")?;
            Keypair::ed25519_from_bytes(decoded_key).context("could not import secret key")?
        }
        // if neither seed nor secrete key were provided, generate keypair from random seed
        None => Keypair::generate_ed25519(),
    };

    Ok(id_keys)
}

fn build_transport(id_keys: Keypair) -> Boxed<(PeerId, StreamMuxerBox)> {
    let mut quic_config = QuicConfig::new(&id_keys);
    quic_config.support_draft_29 = true;
    QuicTransport::new(quic_config)
        .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
        .boxed()
}

fn create_swarm(id_keys: Keypair, cfg: LibP2PConfig) -> Swarm<Behaviour> {
    let local_peer_id = PeerId::from(id_keys.public());
    info!("Local peer id: {:?}.", local_peer_id,);

    // configure Kademlia Memory Store
    let kad_store = MemoryStore::with_config(
        local_peer_id,
        MemoryStoreConfig {
            max_records: cfg.kademlia.max_kad_record_number, // ~2hrs
            max_value_bytes: cfg.kademlia.max_kad_record_size + 1,
            max_providers_per_key: usize::from(cfg.kademlia.record_replication_factor), // Needs to match the replication factor, per libp2p docs
            max_provided_keys: cfg.kademlia.max_kad_provided_keys,
        },
    );

    // create Kademlia Config
    let mut kad_cfg = KademliaConfig::default();
    kad_cfg
        .set_publication_interval(cfg.kademlia.publication_interval)
        .set_replication_interval(cfg.kademlia.record_replication_interval)
        .set_replication_factor(cfg.kademlia.record_replication_factor)
        .set_connection_idle_timeout(cfg.kademlia.connection_idle_timeout)
        .set_query_timeout(cfg.kademlia.query_timeout)
        .set_parallelism(cfg.kademlia.query_parallelism)
        .set_caching(KademliaCaching::Enabled {
            max_peers: cfg.kademlia.caching_max_peers,
        })
        .disjoint_query_paths(cfg.kademlia.disjoint_query_paths);

    // create Indetify Protocol Config
    let identify_cfg = identify::Config::new(cfg.identify_protocol_version, id_keys.public())
        .with_agent_version(cfg.identify_agent_version);

    // create AutoNAT Server Config
    let autonat_cfg = autonat::Config {
        only_global_ips: cfg.autonat_only_global_ips,
        ..Default::default()
    };

    let behaviour = Behaviour {
        kademlia: Kademlia::with_config(local_peer_id, kad_store, kad_cfg),
        identify: identify::Behaviour::new(identify_cfg),
        auto_nat: autonat::Behaviour::new(local_peer_id, autonat_cfg),
        ping: ping::Behaviour::new(PingConfig::new()),
    };

    SwarmBuilder::with_tokio_executor(build_transport(id_keys), behaviour, local_peer_id).build()
}

async fn run() -> Result<()> {
    let opts = CliOpts::parse();
    let cfg_path = &opts.config;
    let cfg: RuntimeConfig = confy::load_path(cfg_path)
        .context(format!("Failed to load configuration from path {cfg_path}"))?;

    let (log_lvl, parse_err) = parse_log_lvl(&cfg.log_level, Level::INFO);
    // set json trace format
    if cfg.log_format_json {
        tracing::subscriber::set_global_default(json_subscriber(log_lvl))
            .expect("global json subscriber to be set");
    } else {
        tracing::subscriber::set_global_default(default_subscriber(log_lvl))
            .expect("global default subscriber to be set");
    }
    if let Some(err) = parse_err {
        warn!("Using default log level: {err}");
    }

    info!("Bootstrap node starting ...");
    let id_keys = generate_id_keys(cfg.secret_key.clone())?;
    let mut swarm = create_swarm(id_keys, (&cfg).into());

    // Listen on all interfaces
    let listen_addr = Multiaddr::empty()
        .with(Protocol::from(Ipv4Addr::UNSPECIFIED))
        .with(Protocol::Udp(cfg.libp2p_port))
        .with(Protocol::QuicV1);
    swarm.listen_on(listen_addr)?;

    tokio::spawn(async move {
        loop {
            match swarm.next().await.expect("Stream to be infinite.") {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Relay is listening on {address:?}");
                }

                SwarmEvent::Behaviour(BehaviourEvent::Identify(event)) => match event {
                    IdentifyEvent::Received {
                        peer_id,
                        info: Info { listen_addrs, protocol_version, .. },
                    } => {
                        debug!("Identity Received from: {peer_id:?} on listen address: {listen_addrs:?}");

                        // only keep records of nodes with the same application-specific
                        // version of the protocol family used by the peer
                        if protocol_version == cfg.libp2p_identify_protocol {
                        for addr in listen_addrs {
                            swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                    }}

                    IdentifyEvent::Sent { peer_id } => {
                        debug!("Identity Sent event to: {peer_id:?}");
                    }

                    _ => {}
                },

                _ => {}
            }
        }
    }).await.context("Event loop failed to run")
}

#[tokio::main]
async fn main() -> Result<()> {
    run().await.map_err(|err| {
        error!("{err}");
        err
    })
}

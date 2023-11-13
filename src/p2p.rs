use anyhow::{Context, Result};
use libp2p::{
    autonat, identify,
    identity::Keypair,
    kad::{self, store::MemoryStore, Mode},
    multiaddr::Protocol,
    ping,
    swarm::NetworkBehaviour,
    Multiaddr, PeerId, SwarmBuilder,
};
use multihash::Hasher;
use tokio::sync::mpsc;

mod client;
mod event_loop;

use crate::{
    p2p::client::{Client, Command},
    types::{LibP2PConfig, SecretKey},
};
use event_loop::EventLoop;
use tracing::info;

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    identify: identify::Behaviour,
    auto_nat: autonat::Behaviour,
    ping: ping::Behaviour,
}

pub fn init(cfg: LibP2PConfig, id_keys: Keypair) -> Result<(Client, EventLoop)> {
    let local_peer_id = PeerId::from(id_keys.public());
    info!(
        "Local Peer ID: {:?}. Public key: {:?}.",
        local_peer_id,
        id_keys.public()
    );

    let mut swarm = SwarmBuilder::with_existing_identity(id_keys)
        .with_tokio()
        .with_quic()
        .with_dns()?
        .with_behaviour(|key| {
            // create new Kademlia Memory Store
            let kad_store = MemoryStore::new(key.public().to_peer_id());
            // create Kademlia Config
            let mut kad_cfg = kad::Config::default();
            kad_cfg.set_query_timeout(cfg.kademlia.query_timeout);

            // create Identify Protocol Config
            let identify_cfg = identify::Config::new(cfg.identify_protocol_version, key.public())
                .with_agent_version(cfg.identify_agent_version);

            // create AutoNAT Server Config
            let autonat_cfg = autonat::Config {
                only_global_ips: cfg.autonat_only_global_ips,
                ..Default::default()
            };

            Behaviour {
                kademlia: kad::Behaviour::with_config(
                    key.public().to_peer_id(),
                    kad_store,
                    kad_cfg,
                ),
                identify: identify::Behaviour::new(identify_cfg),
                auto_nat: autonat::Behaviour::new(local_peer_id, autonat_cfg),
                ping: ping::Behaviour::new(ping::Config::new()),
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(cfg.connection_idle_timeout))
        .build();

    // enable Kademlila Server mode
    swarm.behaviour_mut().kademlia.set_mode(Some(Mode::Server));

    // create channel for Event Loop Commands
    let (command_sender, command_receiver) = mpsc::channel::<Command>(1000);

    Ok((
        Client::new(command_sender),
        EventLoop::new(swarm, command_receiver, cfg.bootstrap_interval),
    ))
}

pub fn keypair(cfg: LibP2PConfig) -> Result<(Keypair, String)> {
    let keypair = match cfg.secret_key {
        // if seed is provided, generate secret key from seed
        Some(SecretKey::Seed { seed }) => {
            let digest = multihash::Sha3_256::digest(seed.as_bytes());
            Keypair::ed25519_from_bytes(digest).context("Error generating secret key from seed")?
        }
        // import secret key, if provided
        Some(SecretKey::Key { key }) => {
            let mut decoded_key = [0u8; 32];
            hex::decode_to_slice(key.into_bytes(), &mut decoded_key)
                .context("Error decoding secret key from config.")?;
            Keypair::ed25519_from_bytes(decoded_key).context("Error importing secret key.")?
        }
        // if neither seed nor secret key were provided,
        // generate secret key from random seed
        None => Keypair::generate_ed25519(),
    };

    let peer_id = PeerId::from(keypair.public()).to_string();
    Ok((keypair, peer_id))
}

pub fn extract_ip(multiaddress: Multiaddr) -> Option<String> {
    for protocol in &multiaddress {
        match protocol {
            Protocol::Ip4(ip) => return Some(ip.to_string()),
            Protocol::Ip6(ip) => return Some(ip.to_string()),
            _ => continue,
        }
    }
    None
}

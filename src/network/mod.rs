mod client;
mod event_loop;

use anyhow::{Context, Result};
use libp2p::{
    autonat::{Behaviour as AutoNAT, Config as AutoNATConfig},
    core::muxing::StreamMuxerBox,
    dns::TokioDnsConfig,
    identify::{Behaviour as Identify, Config as IdentifyConfig},
    identity::Keypair,
    kad::{store::MemoryStore, Kademlia, KademliaConfig, Mode},
    multiaddr::Protocol,
    ping::{Behaviour as Ping, Config as PingConfig},
    quic::{tokio::Transport as TokioQuic, Config as QuicConfig},
    swarm::{NetworkBehaviour, SwarmBuilder},
    Multiaddr, PeerId, Transport,
};
use multihash::Hasher;
use tokio::sync::mpsc;

use event_loop::EventLoop;
use tracing::info;

use crate::{
    network::client::{Client, Command},
    types::{LibP2PConfig, SecretKey},
};

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    kademlia: Kademlia<MemoryStore>,
    identify: Identify,
    auto_nat: AutoNAT,
    ping: Ping,
}

pub fn init(cfg: LibP2PConfig, id_keys: Keypair) -> Result<(Client, EventLoop)> {
    let local_peer_id = PeerId::from(id_keys.public());
    info!(
        "Local Peer ID: {:?}. Public key: {:?}.",
        local_peer_id,
        id_keys.public()
    );

    // create Transport
    let transport = {
        let config = QuicConfig::new(&id_keys);
        let quic = TokioQuic::new(config)
            .map(|(peer_id, muxer), _| (peer_id, StreamMuxerBox::new(muxer)))
            .boxed();
        TokioDnsConfig::system(quic)?.boxed()
    };
    // create new Kademlia Memory Store
    let kad_store = MemoryStore::new(local_peer_id);
    // create Kademlia Config
    let mut kad_cfg = KademliaConfig::default();
    kad_cfg
        .set_connection_idle_timeout(cfg.kademlia.connection_idle_timeout)
        .set_query_timeout(cfg.kademlia.query_timeout);
    // create Identify Protocol Config
    let identify_cfg = IdentifyConfig::new(cfg.identify_protocol_version, id_keys.public())
        .with_agent_version(cfg.identify_agent_version);
    // create AutoNAT Server Config
    let autonat_cfg = AutoNATConfig {
        only_global_ips: cfg.autonat_only_global_ips,
        ..Default::default()
    };
    // initialize Network Behaviour
    let mut behaviour = Behaviour {
        kademlia: Kademlia::with_config(local_peer_id, kad_store, kad_cfg),
        identify: Identify::new(identify_cfg),
        auto_nat: AutoNAT::new(local_peer_id, autonat_cfg),
        ping: Ping::new(PingConfig::new()),
    };

    // Enable Kademlila Server mode
    behaviour.kademlia.set_mode(Some(Mode::Server));
    // build the Swarm
    // Swarm connects the lower transport logic
    // with the higher layer network behaviour logic
    let swarm = SwarmBuilder::with_tokio_executor(transport, behaviour, local_peer_id).build();
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

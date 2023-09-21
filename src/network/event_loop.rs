use anyhow::Result;
use async_std::stream::StreamExt;
use libp2p::{
    autonat::Event as AutoNATEvent,
    identify::{Event as IdentifyEvent, Info},
    kad::{BootstrapOk, KademliaEvent, QueryId, QueryResult},
    multiaddr::Protocol,
    swarm::{derive_prelude::Either, ConnectionError, SwarmEvent},
    PeerId, Swarm,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::{mpsc, oneshot},
    time::{interval_at, Instant, Interval},
};
use tracing::{debug, trace};

use super::{client::Command, Behaviour, BehaviourEvent};

enum QueryChannel {
    Bootstrap(oneshot::Sender<Result<()>>),
}

// BootstrapState keeps track of all things bootstrap related
struct BootstrapState {
    // referring to this initial bootstrap process,
    // one that runs when this node starts up
    is_startup_done: bool,
    // timer that is responsible for firing periodic bootstraps
    timer: Interval,
}

pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    command_receiver: mpsc::Receiver<Command>,
    pending_kad_queries: HashMap<QueryId, QueryChannel>,
    pending_kad_routing: HashMap<PeerId, oneshot::Sender<Result<()>>>,
    bootstrap: BootstrapState,
}

type IoError = Either<std::io::Error, std::io::Error>;
type IoOrVoid = Either<IoError, void::Void>;
type StreamError = Either<IoOrVoid, void::Void>;

impl EventLoop {
    pub fn new(
        swarm: Swarm<Behaviour>,
        command_receiver: mpsc::Receiver<Command>,
        bootstrap_interval: Duration,
    ) -> Self {
        Self {
            swarm,
            command_receiver,
            pending_kad_queries: Default::default(),
            pending_kad_routing: Default::default(),
            bootstrap: BootstrapState {
                is_startup_done: false,
                timer: interval_at(Instant::now() + bootstrap_interval, bootstrap_interval),
            },
        }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.next() => self.handle_event(event.expect("Swarm stream should be infinite")).await,
                command = self.command_receiver.recv() => match command {
                    Some(cmd) => self.handle_command(cmd).await,
                    // command channel closed,
                    // shutting down whole network event loop
                    None => return,
                },
                _ = self.bootstrap.timer.tick() => self.handle_periodic_bootstraps(),
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent, StreamError>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad_event)) => match kad_event {
                KademliaEvent::RoutingUpdated {
                    peer,
                    is_new_peer,
                    addresses,
                    old_peer,
                    ..
                } => {
                    debug!("Routing updated. Peer: {peer:?}. Is new Peer: {is_new_peer:?}. Addresses: {addresses:#?}. Old Peer: {old_peer:#?}");
                    if let Some(res_sender) = self.pending_kad_routing.remove(&peer) {
                        _ = res_sender.send(Ok(()))
                    }
                }

                KademliaEvent::OutboundQueryProgressed { id, result, .. } => match result {
                    QueryResult::Bootstrap(bootstrap_result) => match bootstrap_result {
                        Ok(BootstrapOk {
                            peer,
                            num_remaining,
                        }) => {
                            trace!("BootstrapOK event. PeerID: {peer:?}. Num remaining: {num_remaining:?}.");
                            if num_remaining == 0 {
                                if let Some(QueryChannel::Bootstrap(ch)) =
                                    self.pending_kad_queries.remove(&id)
                                {
                                    _ = ch.send(Ok(()));
                                    // we can say that the initial bootstrap at initialization is done
                                    self.bootstrap.is_startup_done = true;
                                }
                            }
                        }
                        Err(err) => {
                            trace!("Bootstrap error event. Error: {err:?}.");
                            if let Some(QueryChannel::Bootstrap(ch)) =
                                self.pending_kad_queries.remove(&id)
                            {
                                _ = ch.send(Err(err.into()));
                            }
                        }
                    },
                    _ => {}
                },
                _ => {}
            },
            SwarmEvent::Behaviour(BehaviourEvent::Identify(identify_event)) => {
                match identify_event {
                    IdentifyEvent::Received {
                        peer_id,
                        info: Info { listen_addrs, .. },
                    } => {
                        debug!("Identity received from: {peer_id:?} on listen address: {listen_addrs:?}");
                        // interested in addresses with actual Multiaddresses
                        // containing proper 'p2p' protocol tag
                        listen_addrs
                            .iter()
                            .filter(|a| a.to_string().contains(Protocol::P2p(peer_id).tag()))
                            .for_each(|a| {
                                self.swarm
                                    .behaviour_mut()
                                    .kademlia
                                    .add_address(&peer_id, a.clone());
                            });
                    }
                    _ => {}
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::AutoNat(autonat_event)) => match autonat_event {
                AutoNATEvent::InboundProbe(e) => {
                    debug!("AutoNAT Inbound Probe: {:#?}", e);
                }
                AutoNATEvent::OutboundProbe(e) => {
                    debug!("AutoNAT Outbound Probe: {:#?}", e);
                }
                AutoNATEvent::StatusChanged { old, new } => {
                    debug!(
                        "AutoNAT Old status: {:#?}. AutoNAT New status: {:#?}",
                        old, new
                    );
                }
            },
            SwarmEvent::ConnectionClosed {
                peer_id,
                endpoint,
                num_established,
                cause,
                ..
            } => {
                trace!("Connection closed. PeerID: {peer_id:?}. Address: {:?}. Num established: {num_established:?}. Cause: {cause:?}.", endpoint.get_remote_address());
                if let Some(cause) = cause {
                    match cause {
                        // remove peers with failed connections
                        ConnectionError::IO(_) | ConnectionError::Handler(_) => {
                            self.swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
                        } // ignore Keep Alive timeout errors
                        // allow redials for this type of error
                        _ => {}
                    }
                }
            }
            SwarmEvent::OutgoingConnectionError { peer_id, .. } => {
                // what error it was, all current ones are pretty critical
                // remove error producing peer from further dialing
                if let Some(peer_id) = peer_id {
                    trace!("Error produced by peer with PeerId: {peer_id:?}");
                    self.swarm.behaviour_mut().kademlia.remove_peer(&peer_id);
                }
            }
            _ => {}
        }
    }

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::StartListening {
                addr,
                response_sender,
            } => {
                _ = match self.swarm.listen_on(addr) {
                    Ok(_) => response_sender.send(Ok(())),
                    Err(err) => response_sender.send(Err(err.into())),
                }
            }
            Command::AddAddress {
                peer_id,
                peer_addr,
                response_sender,
            } => {
                self.swarm
                    .behaviour_mut()
                    .kademlia
                    .add_address(&peer_id, peer_addr);
                self.pending_kad_routing.insert(peer_id, response_sender);
            }
            Command::Bootstrap { response_sender } => {
                match self.swarm.behaviour_mut().kademlia.bootstrap() {
                    Ok(query_id) => {
                        self.pending_kad_queries
                            .insert(query_id, QueryChannel::Bootstrap(response_sender));
                    }
                    // no available peers for bootstrap
                    // send error immediately through response channel
                    Err(err) => {
                        _ = response_sender.send(Err(err.into()));
                    }
                }
            }
        }
    }

    fn handle_periodic_bootstraps(&mut self) {
        // periodic bootstraps should only start after the initial one is done
        if self.bootstrap.is_startup_done {
            _ = self.swarm.behaviour_mut().kademlia.bootstrap();
        }
    }
}

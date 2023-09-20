use anyhow::Result;
use libp2p::{futures::channel::oneshot, PeerId, Swarm};
use std::collections::HashMap;
use tokio::sync::mpsc;

use super::{client::Command, Behaviour};

pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    command_receiver: mpsc::Receiver<Command>,
    pending_kad_routing: HashMap<PeerId, oneshot::Sender<Result<()>>>,
}

impl EventLoop {
    pub fn new(swarm: Swarm<Behaviour>, command_receiver: mpsc::Receiver<Command>) -> Self {
        Self {
            swarm,
            command_receiver,
            pending_kad_routing: Default::default(),
        }
    }
}

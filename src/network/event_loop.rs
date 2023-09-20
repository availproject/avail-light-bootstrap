use anyhow::Result;
use async_std::stream::StreamExt;
use libp2p::{
    futures::channel::oneshot,
    swarm::{derive_prelude::Either, SwarmEvent},
    PeerId, Swarm,
};
use std::{collections::HashMap, time::Duration};
use tokio::{
    sync::mpsc,
    time::{interval_at, Instant, Interval},
};

use super::{client::Command, Behaviour, BehaviourEvent};

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
                // _ = self.bootstrap.timer.tick() => self.handle_periodic_bootstraps(),
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent, StreamError>) {}

    async fn handle_command(&mut self, command: Command) {}

    fn handle_periodic_bootstraps(mut self) {}
}

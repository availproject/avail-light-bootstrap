use anyhow::{Context, Error, Result};
use libp2p::{Multiaddr, PeerId};
use tokio::sync::{mpsc, oneshot};

#[derive(Clone)]
pub struct Client {
    command_sender: mpsc::Sender<Command>,
}

impl Client {
    pub fn new(command_sender: mpsc::Sender<Command>) -> Self {
        Self { command_sender }
    }

    pub async fn start_listening(&self, addr: Multiaddr) -> Result<(), Error> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::StartListening {
                addr,
                response_sender,
            })
            .await
            .context("Command receiver should not be dropped")?;
        response_receiver
            .await
            .context("Sender not to be dropped")?
    }

    pub async fn add_address(&self, peer_id: PeerId, peer_addr: Multiaddr) -> Result<(), Error> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::AddAddress {
                peer_id,
                peer_addr,
                response_sender,
            })
            .await
            .context("Command receiver should not be dropped.")?;
        response_receiver
            .await
            .context("Sender not to be dropped.")?
    }

    pub async fn bootstrap(&self, nodes: Vec<(PeerId, Multiaddr)>) -> Result<(), Error> {
        for (peer, addr) in nodes {
            self.add_address(peer, addr).await?;
        }

        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::Bootstrap { response_sender })
            .await
            .context("Command receiver should not be dropped.")?;
        response_receiver
            .await
            .context("Sender not to be dropped.")?
    }
}

#[derive(Debug)]
pub enum Command {
    StartListening {
        addr: Multiaddr,
        response_sender: oneshot::Sender<Result<()>>,
    },
    AddAddress {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        response_sender: oneshot::Sender<Result<()>>,
    },
    Bootstrap {
        response_sender: oneshot::Sender<Result<()>>,
    },
    NetworkObservabilityDump,
}

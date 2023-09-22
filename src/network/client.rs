use anyhow::{Context, Error, Result};
use libp2p::{
    kad::{Addresses, EntryView, KBucketKey},
    Multiaddr, PeerId,
};
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

    pub async fn bootstrap(&self) -> Result<(), Error> {
        // bootstrapping is impossible on an empty DHT table
        // at least one node is required to be known, so check
        let (count_res_sender, count_res_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::GetDHTEntries {
                response_sender: count_res_sender,
            })
            .await
            .context("Command receiver should not be dropped while counting dht peers.")?;

        let counted_peers = count_res_receiver.await?.len();
        // for a bootstrap to succeed, we need at least 1 peer in our DHT
        if counted_peers < 1 {
            // we'll have to wait, until some one successfully connects us
            let (connection_res_sender, connection_res_receiver) = oneshot::channel();
            self.command_sender
                .send(Command::WaitIncomingConnection {
                    response_sender: connection_res_sender,
                })
                .await
                .context("Command receiver should not be dropped while waiting on connection.")?;
            // wait here
            _ = connection_res_receiver.await?;
        }

        // proceed to bootstrap only if connected with someone
        let (boot_res_sender, boot_res_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::Bootstrap {
                response_sender: boot_res_sender,
            })
            .await
            .context("Command receiver should not be dropped while bootstrapping.")?;
        boot_res_receiver
            .await
            .context("Sender not to be dropped while bootstrapping.")?
    }

    pub async fn get_dht_entries(&self) -> Result<Vec<EntryView<KBucketKey<PeerId>, Addresses>>> {
        let (response_sender, response_receiver) = oneshot::channel();
        self.command_sender
            .send(Command::GetDHTEntries { response_sender })
            .await
            .context("Command receiver not to be dropped.")?;
        response_receiver.await.context("Sender not to be dropped.")
    }
}

#[derive(Debug)]
pub enum Command {
    StartListening {
        addr: Multiaddr,
        response_sender: oneshot::Sender<Result<()>>,
    },
    Bootstrap {
        response_sender: oneshot::Sender<Result<()>>,
    },
    WaitIncomingConnection {
        response_sender: oneshot::Sender<()>,
    },
    GetDHTEntries {
        response_sender: oneshot::Sender<Vec<EntryView<KBucketKey<PeerId>, Addresses>>>,
    },
}

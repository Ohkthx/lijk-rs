use std::sync::mpsc;

use anyhow::{Result, bail};

use crate::{Packet, PacketHandler, connection::ConnectionError};

/// Local connection that uses MPSC to communicate locally.
pub struct LocalConnection {
    tx: Option<mpsc::Sender<Packet>>,
    rx: Option<mpsc::Receiver<Packet>>,
}

impl LocalConnection {
    /// Creates a new local connection.
    pub(crate) fn new() -> Self {
        Self { tx: None, rx: None }
    }

    /// Creates the receiver for the connection.
    pub(crate) fn create_rx(&mut self) -> Result<mpsc::Receiver<Packet>> {
        if self.tx.is_some() {
            bail!(ConnectionError::DuplicateConnection);
        }

        let (tx, rx) = mpsc::channel::<Packet>();
        self.tx = Some(tx);
        Ok(rx)
    }

    /// Sets the receiver for the connection.
    pub(crate) fn set_rx(&mut self, rx: mpsc::Receiver<Packet>) -> Result<()> {
        if self.rx.is_some() {
            bail!(ConnectionError::DuplicateConnection);
        }

        self.rx = Some(rx);
        Ok(())
    }
}

impl PacketHandler for LocalConnection {
    fn send(&self, packet: Packet) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(packet);
        }
    }

    fn try_recv(&self) -> Result<Option<Packet>> {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(packet) => {
                    if packet.get_version() != Packet::VERSION {
                        bail!(ConnectionError::InvalidPacketVersion(packet.get_version()));
                    } else if !packet.is_valid() {
                        bail!(ConnectionError::InvalidPacketLength(packet.get_length()));
                    }

                    Ok(Some(packet))
                }
                Err(mpsc::TryRecvError::Empty) => Ok(None),
                Err(mpsc::TryRecvError::Disconnected) => bail!(ConnectionError::Disconnected),
            }
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }

    fn recv(&self) -> Result<Packet> {
        if let Some(rx) = &self.rx {
            match rx.recv() {
                Ok(packet) => {
                    if packet.get_version() != Packet::VERSION {
                        bail!(ConnectionError::InvalidPacketVersion(packet.get_version()));
                    } else if !packet.is_valid() {
                        bail!(ConnectionError::InvalidPacketLength(packet.get_length()));
                    }

                    Ok(packet)
                }
                Err(_) => bail!(ConnectionError::Disconnected),
            }
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }
}

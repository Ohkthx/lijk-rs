use std::sync::mpsc;

use anyhow::{Result, bail};

use crate::net::PacketError;

use super::socket::SocketHandler;
use super::storage::ClientStorage;
use super::{ConnectionError, Deliverable, INVALID_CLIENT_ID, Packet, PacketType, SERVER_ID};

/// Local connection that uses MPSC to communicate locally.
pub(crate) struct LocalSocket {
    id: u32,         // Unique identifier for the connection.
    is_server: bool, // Used to test if a server or not.

    tx: Option<mpsc::Sender<Packet>>,   // Sender for the connection.
    rx: Option<mpsc::Receiver<Packet>>, // Receiver for the connection.

    clients: ClientStorage<u32>, // Information about the clients connected to the server.
}

impl LocalSocket {
    /// Maxmium clients for the server.
    pub(crate) const MAX_CLIENTS: u32 = 1;

    /// Creates a new local connection.
    pub(crate) fn new(is_server: bool) -> Self {
        let (id, offset) = if is_server {
            (SERVER_ID, 1)
        } else {
            (INVALID_CLIENT_ID, 0)
        };

        Self {
            id,
            is_server,

            tx: None,
            rx: None,

            clients: ClientStorage::new(Self::MAX_CLIENTS, offset),
        }
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

    /// Obtains the address of the socket.
    #[inline]
    pub(crate) fn address() -> &'static str {
        "localhost"
    }

    /// Obtains the ID of the socket.
    #[inline]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    /// Checks if the connection is a server.
    #[inline]
    pub(crate) fn is_server(&self) -> bool {
        self.is_server
    }

    /// Obtains the last sequence ID for the client.
    #[inline]
    pub(crate) fn last_sequence_id(&self, client_id: u32) -> Option<&u32> {
        self.clients.get_sequence(client_id)
    }

    /// Returns the remote IDs connected to the server.
    #[inline]
    pub(crate) fn remote_ids(&self) -> Vec<u32> {
        self.clients.addr_iter().map(|(id, _)| id).collect()
    }

    /// Adds a new client, returning the client's ID.
    fn add_client(&mut self, addr: u32) -> Result<u32> {
        self.clients.add(addr)
    }

    /// Removes the client from the server.
    fn remove_client(&mut self, client_id: u32) {
        self.clients.remove(client_id);
    }

    /// Disconnects a client from the server.
    /// If `notify` is true, the client will be notified of the disconnection.
    /// Otherwise, the client will be silently disconnected.
    pub(crate) fn disconnect_client(&mut self, uuid: u32, notify: bool) -> Result<()> {
        if notify && self.is_server() {
            // Send a disconnect packet to the client.
            let to_send = Packet::new(PacketType::Disconnect, self.id);
            self.send(Deliverable::new(uuid, to_send))?;
        }

        self.remove_client(uuid);
        Ok(())
    }

    /// Ensure the packet is valid. Additionally assigns an ID to a new client.
    fn validate_packet(&mut self, packet: &mut Packet) -> Result<()> {
        if packet.get_type() == PacketType::Connect {
            if packet.get_sender() == INVALID_CLIENT_ID && self.is_server() {
                // Server side, create a new ID for the client.
                let p_id = self.clients.next_id();
                let client_id = self.add_client(p_id)?;
                assert!(
                    client_id == p_id,
                    "Client ID ({client_id}) is not the same as the next ID ({p_id})"
                );
                packet.set_sender(client_id);
            } else if !self.is_server() {
                const ID_SIZE: usize = size_of::<u32>();
                let raw_id = packet.get_payload();
                if raw_id.len() == ID_SIZE {
                    self.id = u32::from_le_bytes(raw_id.try_into().map_err(|_| {
                        ConnectionError::InvalidPacketPayload(
                            "ID for Connection (Invalid)".to_string(),
                        )
                    })?);
                    self.clients
                        .insert(packet.get_sender(), packet.get_sender());
                } else {
                    bail!(ConnectionError::InvalidPacketPayload(
                        "ID for Connection (Missing)".to_string()
                    ));
                }
            }
        }

        Ok(())
    }
}

impl SocketHandler for LocalSocket {
    #[inline]
    fn send(&mut self, Deliverable { to, mut packet, .. }: Deliverable) -> Result<()> {
        if !(packet.get_sender() == INVALID_CLIENT_ID && packet.get_type() == PacketType::Connect) {
            if let Some(seq) = self.clients.get_sequence_mut(to) {
                *seq += 1;
                packet.set_sequence(*seq);
            } else {
                bail!(ConnectionError::NotConnected);
            };
        }

        if let Some(tx) = &self.tx {
            let _ = tx.send(packet);
        } else {
            bail!(ConnectionError::NotConnected);
        }

        Ok(())
    }

    #[inline]
    fn try_recv(&mut self) -> Result<Option<Packet>> {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(mut packet) => {
                    if let Err(why) = self.validate_packet(&mut packet) {
                        if let Some(ConnectionError::TooManyConnections) =
                            why.downcast_ref::<ConnectionError>()
                        {
                            let mut packet = Packet::new(PacketType::Error, self.id);
                            let mut bytes = vec![PacketError::TooManyConnections as u8];
                            bytes.extend_from_slice("Too many connections".as_bytes());
                            packet.set_payload(&bytes);

                            if let Some(tx) = &self.tx {
                                let _ = tx.send(packet);
                            }
                            return Ok(None);
                        }

                        bail!(why);
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

    #[inline]
    fn recv(&mut self) -> Result<Option<Packet>> {
        if let Some(rx) = &self.rx {
            match rx.recv() {
                Ok(mut packet) => {
                    if let Err(why) = self.validate_packet(&mut packet) {
                        if let Some(ConnectionError::TooManyConnections) =
                            why.downcast_ref::<ConnectionError>()
                        {
                            let mut packet = Packet::new(PacketType::Error, self.id);
                            let mut bytes = vec![PacketError::TooManyConnections as u8];
                            bytes.extend_from_slice("Too many connections".as_bytes());
                            packet.set_payload(&bytes);

                            if let Some(tx) = &self.tx {
                                let _ = tx.send(packet);
                            }
                            return Ok(None);
                        }

                        bail!(why);
                    }

                    Ok(Some(packet))
                }
                Err(_) => bail!(ConnectionError::Disconnected),
            }
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }
}

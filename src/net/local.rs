use std::{collections::HashSet, sync::mpsc};

use anyhow::{Result, bail};
use uuid::Uuid;

use super::{ConnectionError, Deliverable, Packet, PacketType, socket::SocketHandler};

/// Local connection that uses MPSC to communicate locally.
pub(crate) struct LocalSocket {
    uuid: Uuid,       // Unique identifier for the connection.
    is_server: bool,  // Used to test if a server or not.
    sequence_id: u32, // Used to track the last sequence ID for the socket.

    tx: Option<mpsc::Sender<Packet>>,   // Sender for the connection.
    rx: Option<mpsc::Receiver<Packet>>, // Receiver for the connection.

    remotes: HashSet<Uuid>, // Set of remote UUIDs currently connected to.
}

impl LocalSocket {
    /// Creates a new local connection.
    pub(crate) fn new(is_server: bool) -> Self {
        let uuid = if is_server {
            Uuid::new_v4()
        } else {
            Uuid::nil()
        };

        Self {
            uuid,
            is_server,
            sequence_id: 0,
            tx: None,
            rx: None,
            remotes: HashSet::new(),
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

    /// Obtains the last sequence ID for the connection.
    #[inline]
    pub(crate) fn last_sequence_id(&self) -> u32 {
        self.sequence_id
    }

    /// Increments the sequence ID for the connection. Returning the previous ID.
    fn increment_sequence_id(&mut self) -> u32 {
        let id = self.sequence_id;
        self.sequence_id = self.sequence_id.wrapping_add(1);
        id
    }

    /// Obtains the address of the socket.
    #[inline]
    pub(crate) fn address() -> &'static str {
        "localhost"
    }

    /// Obtains the UUID of the socket.
    #[inline]
    pub(crate) fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Checks if the connection is a server.
    #[inline]
    pub(crate) fn is_server(&self) -> bool {
        self.is_server
    }

    /// Returns the remote UUIDs connected to the server.
    #[inline]
    pub(crate) fn remote_uuids(&self) -> Vec<Uuid> {
        self.remotes.iter().copied().collect()
    }

    /// Removes a client from the address and UUID maps.
    fn remove_client(&mut self, uuid: Uuid) {
        // Remove the client from the remote list.
        self.remotes
            .retain(|&v| v != uuid && !v.is_nil() && v != self.uuid);
    }

    /// Disconnects a client from the server.
    /// If `notify` is true, the client will be notified of the disconnection.
    /// Otherwise, the client will be silently disconnected.
    pub(crate) fn disconnect_client(&mut self, uuid: Uuid, notify: bool) -> Result<()> {
        if notify && self.is_server() {
            // Send a disconnect packet to the client.
            let to_send = Packet::new(PacketType::Disconnect, self.uuid);
            self.send(Deliverable::new(uuid, to_send))?;
        }

        self.remove_client(uuid);
        Ok(())
    }

    /// Ensure the packet is valid. Additionally assigns an UUID to a new client.
    fn validate_packet(&mut self, packet: &mut Packet) -> Result<()> {
        if packet.get_type() == PacketType::Connect {
            if packet.get_source().is_nil() && self.is_server() {
                // Server side, create a new UUID for the client.
                packet.set_source(Uuid::new_v4());
                self.remotes.insert(packet.get_source());
            } else if !self.is_server() {
                // Client side, check if the UUID is valid.
                const UUID_SIZE: usize = size_of::<Uuid>();
                let raw_uuid = packet.get_payload();
                if raw_uuid.len() == UUID_SIZE {
                    self.uuid = Uuid::from_slice(raw_uuid).map_err(|_| {
                        ConnectionError::InvalidPacketPayload(
                            "UUID for Connection (Invalid)".to_string(),
                        )
                    })?;
                } else {
                    bail!(ConnectionError::InvalidPacketPayload(
                        "UUID for Connection (Missing)".to_string()
                    ));
                }
            }
        }

        Ok(())
    }
}

impl SocketHandler for LocalSocket {
    #[inline]
    fn send(&mut self, Deliverable { mut packet, .. }: Deliverable) -> Result<()> {
        packet.set_sequence(self.increment_sequence_id());
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
                    self.validate_packet(&mut packet)?;
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
    fn recv(&mut self) -> Result<Packet> {
        if let Some(rx) = &self.rx {
            match rx.recv() {
                Ok(mut packet) => {
                    self.validate_packet(&mut packet)?;
                    Ok(packet)
                }
                Err(_) => bail!(ConnectionError::Disconnected),
            }
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }
}

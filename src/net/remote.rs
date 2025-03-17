use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, UdpSocket};

use anyhow::{Result, bail};
use uuid::Uuid;

use super::packet::RawPacket;
use super::{ConnectionError, Deliverable, Packet, PacketType, socket::SocketHandler};

/// Remote connection that uses UDP to communicate with a remote server or client.
pub(crate) struct RemoteSocket {
    socket: UdpSocket,           // Raw socket.
    uuid: Uuid,                  // Unique identifier for the socket.
    local_addr: String,          // Local address for the socket.
    remote_addr: Option<String>, // Remote address for the socket. Only set for clients.

    addr_uuid: HashMap<SocketAddr, Uuid>, // Maps socket address to UUID.
    uuid_addr: HashMap<Uuid, SocketAddr>, // Maps UUID to socket address.
    remotes: HashSet<Uuid>,               // Set of remote UUIDs currently connected to the server.

    buffer: [u8; 1024], // Buffer for receiving data.
    nonblocking: bool,  // Used to track if the socket is in non-blocking mode.
    sequence_id: u32,   // Used to track the last sequence ID for the socket.
}

impl RemoteSocket {
    /// Default addresses for the server.
    pub(crate) const DEFAULT_SERVER_ADDR: &'static str = "127.0.0.1:31013";
    /// Default address for the client to bind to. This is used when the client does not have a specific address.
    pub(crate) const DEFAULT_CLIENT_ADDR: &'static str = "0.0.0.0:0";

    /// Creates a new remote connection with the given address.
    pub(crate) fn new(address: Option<String>) -> Result<Self> {
        let (bind_addr, uuid) = if address.is_some() {
            (Self::DEFAULT_CLIENT_ADDR, Uuid::nil())
        } else {
            (Self::DEFAULT_SERVER_ADDR, Uuid::new_v4())
        };

        // Bind the socket to the address.
        let socket = match UdpSocket::bind(bind_addr) {
            Ok(socket) => socket,
            Err(why) => bail!(ConnectionError::SocketError(why.to_string())),
        };

        let mut connection = Self {
            socket,
            uuid,
            local_addr: bind_addr.to_string(),
            remote_addr: address,

            addr_uuid: HashMap::new(),
            uuid_addr: HashMap::new(),
            remotes: HashSet::new(),

            buffer: [0; 1024],
            nonblocking: false,
            sequence_id: 0,
        };

        // Set the socket to non-blocking mode.
        connection.toggle_nonblocking()?;

        Ok(connection)
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
    pub(crate) fn address(&self) -> &str {
        &self.local_addr
    }

    /// Obtains the UUID of the socket.
    #[inline]
    pub(crate) fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Checks if the connection is a server.
    #[inline]
    pub(crate) fn is_server(&self) -> bool {
        self.remote_addr.is_none()
    }

    /// Returns the remote UUIDs connected to the server.
    #[inline]
    pub(crate) fn remote_uuids(&self) -> Vec<Uuid> {
        self.remotes.iter().copied().collect()
    }

    /// Toggles between blocking and non-blocking modes.
    fn toggle_nonblocking(&mut self) -> Result<()> {
        self.nonblocking = !self.nonblocking;
        if let Err(why) = self.socket.set_nonblocking(self.nonblocking) {
            self.nonblocking = !self.nonblocking; // Reset if an error occurs.
            bail!(ConnectionError::SocketError(why.to_string()));
        }

        Ok(())
    }

    /// Adds a client to the address and UUID maps.
    fn add_client(&mut self, uuid: Uuid, addr: SocketAddr) {
        self.addr_uuid.insert(addr, uuid);
        self.uuid_addr.insert(uuid, addr);
        self.remotes.insert(uuid);
    }

    /// Removes a client from the address and UUID maps.
    fn remove_client(&mut self, uuid: Uuid) {
        // Remove the client from the list.
        if let Some(addr) = self.uuid_addr.remove(&uuid) {
            self.addr_uuid.remove(&addr);
            if let Some(uuid) = self.addr_uuid.remove(&addr) {
                self.uuid_addr.remove(&uuid);
            }
        } else if self.addr_uuid.iter().any(|(_, v)| v == &uuid) {
            // Remove the UUID from the address map.
            self.addr_uuid.retain(|_, v| v != &uuid);
        }

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

    /// Validates the packet to ensure it is signed with the appropriate UUID to Address.
    fn validate_packet(&mut self, sender: SocketAddr, packet: &mut Packet) -> Result<()> {
        if packet.get_version() != Packet::VERSION {
            bail!(ConnectionError::InvalidPacketVersion(packet.get_version()));
        }

        let mut authed = !self.is_server();

        if packet.get_source().is_nil() {
            // Check if a new client connecting, otherwise give it the old UUID.
            if let Some(uuid) = self.addr_uuid.get(&sender) {
                // Existing client.
                packet.set_source(*uuid);
            } else if packet.get_type() == PacketType::Connect {
                // New client connecting, assign it a new UUID.
                let uuid = Uuid::new_v4();
                self.add_client(uuid, sender);
                packet.set_source(uuid);
                authed = true;
            } else {
                // Client is not authenticated. Never sent connect packet.
                bail!(ConnectionError::AuthenticationFailed);
            }
        }

        // Check if the client is authenticated by checking the UUID to address mapping.
        if self.is_server() && !authed {
            if let Some(uuid) = self.addr_uuid.get(&sender) {
                // Check that the client is using the correct UUID.
                authed = packet.get_source() == *uuid;
                if !authed {
                    // UUID does not match address.
                    bail!(ConnectionError::InvalidPacketUuid(
                        *uuid,
                        packet.get_source()
                    ));
                }
            }

            if !authed {
                // Only check the uuid cache if address lookup failed.
                if let Some(cached) = self.uuid_addr.get(&packet.get_source()) {
                    // Check that the UUID is from the correct address.
                    authed = sender == *cached;
                    if !authed {
                        // Address does not match UUID.
                        bail!(ConnectionError::InvalidPacketAddress(
                            cached.to_string(),
                            sender.to_string(),
                        ));
                    }
                } else {
                    // Socket Address and UUID are not in the cache.
                    bail!(ConnectionError::AuthenticationFailed);
                }
            }
        }

        // Assign the UUID to the client when connecting.
        if packet.get_type() == PacketType::Connect && !self.is_server() {
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

        Ok(())
    }
}

impl SocketHandler for RemoteSocket {
    #[inline]
    fn send(&mut self, Deliverable { to, mut packet }: Deliverable) -> Result<()> {
        packet.set_sequence(self.increment_sequence_id());
        let raw: RawPacket = packet.into();
        if let Some(address) = &self.remote_addr {
            // Send to the host / server.
            self.socket.send_to(raw.get_data(), address)?;
        } else if let Some(addr) = self.uuid_addr.get(&to) {
            // Send to a client.
            self.socket.send_to(raw.get_data(), addr)?;
        } else {
            bail!(ConnectionError::NotConnected);
        }

        Ok(())
    }

    #[inline]
    fn try_recv(&mut self) -> Result<Option<Packet>> {
        if !self.nonblocking {
            self.toggle_nonblocking()?;
        }

        match self.socket.recv_from(&mut self.buffer) {
            Ok((size, sender)) => {
                let raw = RawPacket::from(&self.buffer[..size]);
                if !raw.is_valid_len() {
                    bail!(ConnectionError::InvalidPacketLength(raw.get_data().len()));
                }

                let mut packet = raw.into();
                self.validate_packet(sender, &mut packet)?;
                Ok(Some(packet))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, return None.
                Ok(None)
            }
            Err(why) => bail!(ConnectionError::SocketError(why.to_string())),
        }
    }

    #[inline]
    fn recv(&mut self) -> Result<Packet> {
        if self.nonblocking {
            self.toggle_nonblocking()?;
        }

        match self.socket.recv_from(&mut self.buffer) {
            Ok((size, sender)) => {
                let raw = RawPacket::from(&self.buffer[..size]);
                if !raw.is_valid_len() {
                    bail!(ConnectionError::InvalidPacketLength(raw.get_data().len()));
                }

                let mut packet = raw.into();
                self.validate_packet(sender, &mut packet)?;
                Ok(packet)
            }
            Err(why) => bail!(ConnectionError::SocketError(why.to_string())),
        }
    }
}

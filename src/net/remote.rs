use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use crate::flee;

use super::socket::SocketHandler;
use super::storage::ClientStorage;
use super::{
    Deliverable, ErrorPacket, INVALID_CLIENT_ID, NetError, Packet, PacketType, Result, SERVER_ID,
};

/// Remote connection that uses UDP to communicate with a remote server or client.
pub(crate) struct RemoteSocket {
    socket: UdpSocket,           // Raw socket.
    id: u32,                     // ID of the connection.
    local_addr: String,          // Local address for the socket.
    remote_addr: Option<String>, // Remote address for the socket. Only set for clients.

    clients: ClientStorage<SocketAddr>, // Information about the clients connected to the server.

    buffer: [u8; 1024], // Buffer for receiving data.
    nonblocking: bool,  // Used to track if the socket is in non-blocking mode.
}

impl RemoteSocket {
    /// Default addresses for the server.
    pub(crate) const DEFAULT_SERVER_ADDR: &'static str = "127.0.0.1:31013";
    /// Default address for the client to bind to. This is used when the client does not have a specific address.
    pub(crate) const DEFAULT_CLIENT_ADDR: &'static str = "0.0.0.0:0";
    /// Maxmium clients for the server.
    pub(crate) const MAX_CLIENTS: u32 = 256;

    /// Creates a new remote connection with the given address.
    pub(crate) fn new(address: Option<String>) -> Result<Self> {
        let (bind_addr, id, offset) = if address.is_some() {
            (Self::DEFAULT_CLIENT_ADDR, INVALID_CLIENT_ID, 0)
        } else {
            (Self::DEFAULT_SERVER_ADDR, SERVER_ID, 1)
        };

        // Bind the socket to the address.
        let socket = match UdpSocket::bind(bind_addr) {
            Ok(socket) => socket,
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        };

        let mut connection = Self {
            socket,
            id,
            local_addr: bind_addr.to_string(),
            remote_addr: address,

            clients: ClientStorage::new(Self::MAX_CLIENTS, offset),

            buffer: [0; 1024],
            nonblocking: false,
        };

        // Set the socket to non-blocking mode.
        connection.toggle_nonblocking()?;

        Ok(connection)
    }

    /// Obtains the address of the socket.
    #[inline]
    pub(crate) fn address(&self) -> &str {
        &self.local_addr
    }

    /// Obtains the ID of the socket.
    #[inline]
    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    /// Checks if the connection is a server.
    #[inline]
    pub(crate) fn is_server(&self) -> bool {
        self.remote_addr.is_none()
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

    /// Toggles between blocking and non-blocking modes.
    fn toggle_nonblocking(&mut self) -> Result<()> {
        self.nonblocking = !self.nonblocking;
        if let Err(why) = self.socket.set_nonblocking(self.nonblocking) {
            self.nonblocking = !self.nonblocking; // Reset if an error occurs.
            flee!(NetError::SocketError(why.to_string()));
        }

        Ok(())
    }

    /// Adds a new client, returning the client's ID.
    fn add_client(&mut self, addr: SocketAddr) -> Result<u32> {
        let client_id = self.clients.add(addr);
        if client_id == INVALID_CLIENT_ID {
            flee!(NetError::TooManyConnections);
        }

        Ok(client_id)
    }

    /// Removes a client from the address and ID maps.
    fn remove_client(&mut self, client_id: u32) {
        self.clients.remove(client_id);
    }

    /// Disconnects a client from the server.
    /// If `notify` is true, the client will be notified of the disconnection.
    /// Otherwise, the client will be silently disconnected.
    pub(crate) fn disconnect_client(&mut self, client_id: u32, notify: bool) -> Result<()> {
        if notify && self.is_server() {
            // Send a disconnect packet to the client.
            let to_send = Packet::new(PacketType::Disconnect, self.id);
            self.send(Deliverable::new(client_id, to_send))?;
        }

        self.remove_client(client_id);
        Ok(())
    }

    /// Validates the packet to ensure it is signed with the appropriate ID to Address.
    fn validate_packet(&mut self, sender: SocketAddr, packet: &mut Packet) -> Result<()> {
        let mut authed = !self.is_server();

        if packet.get_sender() == INVALID_CLIENT_ID {
            // Check if a new client connecting, otherwise give it the old ID.
            if let Some(id) = self.clients.get_id(&sender) {
                // Existing client.
                packet.set_sender(id);
            } else if packet.get_type() == PacketType::Connect {
                // New client connecting, assign it a new ID.
                let id = self.add_client(sender)?;
                packet.set_sender(id);
                authed = true;
            } else {
                // Client is not authenticated. Never sent connect packet.
                flee!(NetError::NotConnected(self.is_server()));
            }
        }

        // Check if the client is authenticated by checking the ID to address mapping.
        if self.is_server() && !authed {
            if let Some(id) = self.clients.get_id(&sender) {
                // Check that the client is using the correct ID.
                authed = packet.get_sender() == id;
                if !authed {
                    // ID does not match address.
                    flee!(NetError::InvalidPacketSender(id, packet.get_sender()));
                }
            }

            if !authed {
                // Only check the uuid cache if address lookup failed.
                if let Some(cached) = self.clients.get_addr(packet.get_sender()) {
                    // Check that the ID is from the correct address.
                    authed = sender == *cached;
                    if !authed {
                        // Address does not match ID.
                        flee!(NetError::InvalidPacketAddress(
                            cached.to_string(),
                            sender.to_string(),
                        ));
                    }
                } else {
                    // Socket Address and ID are not in the cache.
                    flee!(NetError::NotConnected(self.is_server()));
                }
            }
        }

        // Assign the ID to the client when connecting.
        if packet.get_type() == PacketType::Connect && !self.is_server() {
            const ID_SIZE: usize = size_of::<u32>();
            let raw_id = packet.get_payload();
            if raw_id.len() == ID_SIZE {
                self.id = u32::from_be_bytes(raw_id.try_into().map_err(|_| {
                    NetError::InvalidPacketPayload("ID for Connection (Invalid)".to_string())
                })?);
                self.clients.insert(packet.get_sender(), sender);
            } else {
                flee!(NetError::InvalidPacketPayload(
                    "ID for Connection (Missing)".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Parses a packet from the buffer and the errors associated with it.
    fn parse_packet(&self, size: usize, sender: &SocketAddr) -> Result<Packet> {
        match Packet::try_from(&self.buffer[..size]) {
            Ok(packet) => Ok(packet),
            Err(NetError::InvalidPacketVersion(ex, got)) => {
                self.send_err(
                    sender,
                    ErrorPacket::InvalidPacketVersion,
                    &format!("Expected version: {ex}, got: {got}"),
                )?;
                flee!(NetError::InvalidPacket(
                    Some(usize::from(ex)),
                    usize::from(got),
                    "Invalid Packet Version".to_string()
                ))
            }
            Err(NetError::InvalidPacketSize(ex, got)) => {
                self.send_err(
                    sender,
                    ErrorPacket::InvalidPacketSize,
                    &format!("Expected minimum size: {ex}, got: {got}"),
                )?;
                flee!(NetError::InvalidPacket(
                    Some(ex),
                    got,
                    "Invalid Packet Size".to_string()
                ))
            }
            Err(NetError::InvalidPacketType(value)) => {
                self.send_err(
                    sender,
                    ErrorPacket::InvalidPacketType,
                    &format!("Packet type got: {value}"),
                )?;
                flee!(NetError::InvalidPacket(
                    None,
                    usize::from(value),
                    "Invalid Packet Type".to_string()
                ))
            }
            Err(why) => flee!(why),
        }
    }

    /// Wraps the `send_to` method to send a packet to a specific address.
    fn send_to<T: ToSocketAddrs>(&self, packet: &Packet, addr: &T) -> Result<()> {
        if let Err(why) = self.socket.send_to(&Vec::from(packet), addr) {
            flee!(NetError::SocketError(format!(
                "Unable to send packet: {why}",
            )));
        }

        Ok(())
    }

    /// Sends an error packet to the specified address.
    fn send_err(&self, sender: &SocketAddr, error: ErrorPacket, msg: &str) -> Result<()> {
        let mut packet = Packet::new(PacketType::Error, self.id);
        let mut bytes = vec![error as u8];
        bytes.extend_from_slice(msg.as_bytes());
        packet.set_payload(bytes);
        self.send_to(&packet, &sender)?;
        Ok(())
    }
}

impl SocketHandler for RemoteSocket {
    #[inline]
    fn send(&mut self, Deliverable { to, mut packet }: Deliverable) -> Result<()> {
        if packet.get_sender() != INVALID_CLIENT_ID || packet.get_type() != PacketType::Connect {
            if let Some(seq) = self.clients.get_sequence_mut(to) {
                *seq += 1;
                packet.set_sequence(*seq);
            } else {
                flee!(NetError::NotConnected(self.is_server()));
            };
        }

        if let Some(address) = &self.remote_addr {
            // Send to the host / server.
            self.send_to(&packet, address)?;
        } else if let Some(addr) = self.clients.get_addr(to) {
            // Send to a client.
            self.send_to(&packet, addr)?;
        } else {
            flee!(NetError::NotConnected(self.is_server()));
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
                // Parse the packet and validate it.
                let mut packet = self.parse_packet(size, &sender)?;
                if let Err(why) = self.validate_packet(sender, &mut packet) {
                    if why == NetError::TooManyConnections {
                        self.send_err(
                            &sender,
                            ErrorPacket::TooManyConnections,
                            "Too many connections",
                        )?;
                        return Ok(None);
                    }

                    flee!(why);
                }

                Ok(Some(packet))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, return None.
                Ok(None)
            }
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        }
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<Packet>> {
        if self.nonblocking {
            self.toggle_nonblocking()?;
        }

        match self.socket.recv_from(&mut self.buffer) {
            Ok((size, sender)) => {
                // Parse the packet and validate it.
                let mut packet = self.parse_packet(size, &sender)?;
                if let Err(why) = self.validate_packet(sender, &mut packet) {
                    if why == NetError::TooManyConnections {
                        self.send_err(
                            &sender,
                            ErrorPacket::TooManyConnections,
                            "Too many connections",
                        )?;
                        return Ok(None);
                    }

                    flee!(why);
                }

                Ok(Some(packet))
            }
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        }
    }
}

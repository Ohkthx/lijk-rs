use std::net::SocketAddr;
use std::str::FromStr;

use crate::flee;

use super::{
    ClientAddr, ClientStorage, Deliverable, EntityId, ErrorPacket, INVALID_CLIENT_ID, LocalSocket,
    NetError, Packet, PacketLabel, RemoteSocket, Result, SERVER_ID, storage::StorageError,
};

/// Trait for handling packets.
pub(crate) trait SocketHandler {
    /// Send a packet to the connection.
    #[allow(dead_code)]
    fn send(&mut self, dest: &ClientAddr, packet: Packet) -> Result<()>;
    /// Try to receive a packet from the connection.
    #[allow(dead_code)]
    fn try_recv(&mut self) -> Result<Option<(ClientAddr, Packet)>>;
    /// Waits to receive a packet from the connection.
    #[allow(dead_code)]
    fn recv(&mut self) -> Result<Option<(ClientAddr, Packet)>>;
}

/// Socket type for the connection. Either a remote or local connection.
enum SocketType {
    Remote(Box<RemoteSocket>), // Remote connection that uses UDP to communicate with a client / server.
    Local(LocalSocket),        // Local connection that uses MPSC to communicate locally.
}

impl SocketHandler for SocketType {
    #[inline]
    fn send(&mut self, dest: &ClientAddr, packet: Packet) -> Result<()> {
        match self {
            SocketType::Remote(socket) => socket.send(dest, packet),
            SocketType::Local(socket) => socket.send(dest, packet),
        }
    }

    #[inline]
    fn try_recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        match self {
            SocketType::Remote(socket) => socket.try_recv(),
            SocketType::Local(socket) => socket.try_recv(),
        }
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        match self {
            SocketType::Remote(socket) => socket.recv(),
            SocketType::Local(socket) => socket.recv(),
        }
    }
}

/// Socket for the connection. Used to send and receive packets to a client / server.
/// This is a unified interface for both local and remote connections.
pub struct Socket {
    id: EntityId,                    // Unique identifier for the connection.
    server_addr: Option<ClientAddr>, // The server address for the connection. Only set for clients.
    raw: SocketType, // The socket type for the connection. Either a remote or local connection.
    clients: ClientStorage<ClientAddr>,
}

impl Socket {
    /// Maximum amount of clients for the socket.
    pub const MAX_CLIENTS: EntityId = 256;

    /// Creates a new socket with the given socket type.
    fn new(socket: SocketType, server_addr: Option<ClientAddr>) -> Result<Self> {
        let is_server = server_addr.is_none();
        let offset = EntityId::from(is_server);
        let id = if is_server {
            SERVER_ID
        } else {
            INVALID_CLIENT_ID
        };

        let clients = match ClientStorage::new(offset, Self::MAX_CLIENTS, INVALID_CLIENT_ID) {
            Ok(clients) => clients,
            Err(why) => flee!(NetError::StorageError(why.to_string())),
        };

        Ok(Self {
            id,
            server_addr,
            raw: socket,
            clients,
        })
    }

    /// Creates a new local connection pair.
    pub fn new_local_pair() -> Result<(Self, Self)> {
        let mut server_socket = LocalSocket::new();
        let mut client_socket = LocalSocket::new();

        // Obtain the receivers for both connections.
        let server_rx = server_socket.create_rx()?;
        let client_rx = client_socket.create_rx()?;

        // Set the receivers for both connections.
        server_socket.set_rx(client_rx)?;
        client_socket.set_rx(server_rx)?;

        let server = SocketType::Local(server_socket);
        let client = SocketType::Local(client_socket);

        let server_addr = Some(ClientAddr::Local(SERVER_ID));
        Ok((Self::new(server, None)?, Self::new(client, server_addr)?))
    }

    /// Creates a new remote connection with the given address.
    pub fn new_remote(server_addr: Option<String>) -> Result<Self> {
        // Conver the server address from String to Client.
        let addr = if let Some(address) = server_addr {
            match SocketAddr::from_str(&address) {
                Ok(addr) => Some(ClientAddr::Ip(addr.ip(), addr.port())),
                Err(_) => flee!(NetError::InvalidServerAddress(address)),
            }
        } else {
            None
        };

        let socket = RemoteSocket::new(addr.is_none())?;
        Self::new(SocketType::Remote(Box::new(socket)), addr)
    }

    /// Checks if the socket is a local connection.
    #[inline]
    pub fn is_remote(&self) -> bool {
        match &self.raw {
            SocketType::Remote(_) => true,
            SocketType::Local(_) => false,
        }
    }

    /// Checks if socket is in server mode.
    #[inline]
    fn is_server(&self) -> bool {
        self.server_addr().is_none()
    }

    /// Local address of the socket.
    #[inline]
    pub fn addr(&self) -> &str {
        match &self.raw {
            SocketType::Remote(socket) => socket.address(),
            SocketType::Local(_) => LocalSocket::address(),
        }
    }

    /// Server address of the socket. Only set for clients.
    #[inline]
    pub fn server_addr(&self) -> Option<ClientAddr> {
        self.server_addr
    }

    /// Local ID of the socket.
    #[inline]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Obtains the UUIDs of the remote sockets.
    #[inline]
    pub fn remote_ids(&self) -> Vec<EntityId> {
        self.clients.addr_iter().map(|(id, _)| id).collect()
    }

    /// Obtains the last sequence ID for the connection.
    #[allow(dead_code)]
    #[inline]
    pub fn last_sequence_id(&self, client_id: EntityId) -> Option<&EntityId> {
        self.clients.get_sequence(client_id)
    }

    /// Adds a new client, returning the client's ID.
    fn add_client(&mut self, client: ClientAddr) -> Result<EntityId> {
        match self.clients.add(client) {
            Ok(client_id) => Ok(client_id),
            Err(StorageError::AtCapacity) => flee!(NetError::TooManyConnections),
            Err(why) => flee!(NetError::StorageError(why.to_string())),
        }
    }

    /// Queues a client for removal.
    fn queue_removal(&mut self, client_id: EntityId) {
        self.clients.remove(client_id);
    }

    /// Checks if the sender is valid for the given client ID and address.
    fn valid_sender(&self, client_id: EntityId, addr: &ClientAddr) -> bool {
        if let Some(cached_id) = self.clients.get_id(addr) {
            return client_id == cached_id;
        } else if let Some(cached) = self.clients.get_addr(client_id) {
            return addr == cached;
        }

        false
    }

    /// Handles a packet that has invalid Client ID. Returns `true` if authenticated.
    ///
    /// # Errors
    ///
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::TooManyConnections` if the maximum number of clients has been reached.
    fn handle_invalid_client_packet(
        &mut self,
        sender: &ClientAddr,
        packet: &mut Packet,
    ) -> Result<()> {
        assert!(
            packet.sender() == INVALID_CLIENT_ID,
            "Packet sender must be invalid."
        );

        // Check if a new client connecting, otherwise give it the old ID.
        if let Some(id) = self.clients.get_id(sender) {
            packet.set_sender(id); // Discovered ID from cache.
        } else if packet.label() == PacketLabel::Connect {
            // New client connecting, assign it a new ID.
            let cache_id = if self.is_remote() {
                // Remote connection, assign a new ID.
                self.add_client(*sender)?
            } else {
                // Need to generate an ID that is not INVALID_CLIENT_ID.
                let id = self.clients.next_id();
                self.add_client(ClientAddr::Local(id))?
            };

            packet.set_sender(cache_id);
        } else {
            // Client is not authenticated. Never sent connect packet.
            flee!(NetError::NotConnected(*sender, true));
        }

        Ok(())
    }

    /// Resolves the clients ID from the sender's address or ID.
    ///
    /// # Errors
    ///
    /// - `NetError::InvalidPacketSender` if the sender ID is invalid.
    /// - `NetError::InvalidPacketAddress` if the address is invalid.
    /// - `NetError::NotConnected` if the connection is not established.
    fn handle_client_lookup(&mut self, sender: &ClientAddr, client_id: EntityId) -> Result<()> {
        if let Some(cached_id) = self.clients.get_id(sender) {
            // Check that the client is using the correct ID.
            if client_id == cached_id {
                return Ok(());
            }

            // ID does not match address.
            flee!(NetError::InvalidPacketSender(cached_id, client_id));
        }

        // Only check the cache if address lookup failed.
        if let Some(cached) = self.clients.get_addr(client_id) {
            // Check that the ID is from the correct address.
            if sender == cached {
                return Ok(());
            }

            // Address does not match ID.
            flee!(NetError::InvalidPacketAddress(
                cached.to_string(),
                sender.to_string(),
            ));
        }

        // Socket Address and ID are not in the cache.
        flee!(NetError::NotConnected(*sender, true));
    }

    /// Validates the incoming packet is correct.
    ///
    /// # Errors
    ///
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::TooManyConnections` if the maximum number of clients has been reached.
    /// - `NetError::InvalidPacketSender` if the sender ID is invalid.
    /// - `NetError::InvalidPacketAddress` if the address is invalid.
    /// - `NetError::InvalidPacketPayload` if the payload is invalid.
    fn validate(&mut self, sender: &ClientAddr, packet: &mut Packet) -> Result<()> {
        let mut authed = !self.is_server();

        // Handles a packet with an invalid client ID.
        if packet.sender() == INVALID_CLIENT_ID {
            self.handle_invalid_client_packet(sender, packet)?;
            authed = true; // Would have error out if not authenticated.
        }

        // Check if the sender is in the cache.
        if self.is_server() && !authed {
            self.handle_client_lookup(sender, packet.sender())?;
        }

        // Assign the ID to the client when connecting.
        if packet.label() == PacketLabel::Connect && !self.is_server() {
            const ID_SIZE: usize = size_of::<EntityId>();
            let raw_id = packet.payload();
            if raw_id.len() == ID_SIZE {
                self.id = EntityId::from_be_bytes(raw_id.try_into().map_err(|_| {
                    NetError::InvalidPacketPayload(
                        "Could not parse Client ID from payload".to_string(),
                    )
                })?);

                self.clients.insert(packet.sender(), *sender);
            } else {
                flee!(NetError::InvalidPacketPayload(
                    "Size of payload for Client ID was incorrect".to_string()
                ));
            }
        }

        Ok(())
    }

    /// Disconnects a client from the server and notifies the client if requested.
    ///
    /// # Errors
    ///
    /// - `NetError::NotServer` if the socket is not in server mode.
    /// - `NetError::SelfConnection` if the destination is the same as the source and the packet is not a connect packet.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` if there is a socket error.
    pub fn disconnect_client(&mut self, client_id: EntityId, notify: bool) -> Result<()> {
        if !self.is_server() {
            flee!(NetError::NotServer);
        }

        if notify {
            // Send a disconnect packet to the client.
            let to_send = Packet::new(PacketLabel::Disconnect, self.id());
            self.send(Deliverable::new(client_id, to_send))?;
        }

        self.queue_removal(client_id);
        Ok(())
    }

    /// Sends an error packet to the specified address.
    ///
    /// # Errors
    ///
    /// - `NetError::SelfConnection` if the destination is the same as the source and the packet is not a connect packet.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` if there is a socket error.
    fn send_err(&mut self, to: EntityId, error: ErrorPacket, msg: &str) -> Result<()> {
        let mut packet = Packet::new(PacketLabel::Error, self.id);
        let mut bytes = vec![error as u8];

        bytes.extend_from_slice(msg.as_bytes());
        packet.set_payload(bytes);

        self.send(Deliverable { to, packet })
    }

    /// Sends a packet to the destination UUID. If the packet is a connect packet, it will not check for self connection.
    ///
    /// # Errors
    ///
    /// - `NetError::SelfConnection` if the destination is the same as the source and the packet is not a connect packet.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` if there is a socket error.
    #[allow(dead_code)]
    pub fn send(&mut self, Deliverable { to, mut packet }: Deliverable) -> Result<()> {
        if self.id() == to && packet.label() != PacketLabel::Connect {
            flee!(NetError::SelfConnection);
        }

        // Update the sequence number for the packet if it's not a connect packet.
        if packet.sender() != INVALID_CLIENT_ID || packet.label() != PacketLabel::Connect {
            if let Some(seq) = self.clients.get_sequence_mut(to) {
                *seq += 1;
                packet.set_sequence(*seq);
            } else {
                flee!(NetError::NotConnected(ClientAddr::Local(to), true));
            }
        }

        // Send the packet to the client.
        if let Some(client) = self.clients.get_addr(to) {
            self.raw.send(client, packet)
        } else if let Some(client) = self.server_addr() {
            self.raw.send(&client, packet)
        } else if !self.is_remote() {
            self.raw.send(&ClientAddr::Local(SERVER_ID), packet)
        } else {
            flee!(NetError::NotConnected(ClientAddr::Local(to), true));
        }
    }

    /// Tries to receive a packet from the connection. Returns None if no packet is available.
    ///
    /// # Errors
    ///
    /// - `NetError::InvalidPacket` if the header length, version, or packet type is incorrect.
    /// - `NetError::InvalidPacketSender` if the sender ID is invalid.
    /// - `NetError::InvalidPacketAddress` if the address is invalid.
    /// - `NetError::InvalidPacketPayload` if the payload is invalid.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` occurs if cannot toggle nonblocking mode or unknown error.
    /// - `NetError::Disconnected` if the connection is disconnected.
    #[allow(dead_code)]
    pub fn try_recv(&mut self) -> Result<Option<Packet>> {
        match self.raw.try_recv() {
            Ok(Some((client, mut packet))) => {
                if let Err(why) = self.validate(&client, &mut packet) {
                    if why == NetError::TooManyConnections
                        && self.valid_sender(packet.sender(), &client)
                    {
                        self.send_err(
                            packet.sender(),
                            ErrorPacket::TooManyConnections,
                            "Too many connections",
                        )?;
                        return Ok(None);
                    }

                    flee!(why);
                }

                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(why) => flee!(why),
        }
    }

    /// Waits to receive a packet from the connection. Returns an error if a connection issue occurs.
    ///
    /// # Errors
    ///
    /// - `NetError::InvalidPacket` if the header length, version, or packet type is incorrect.
    /// - `NetError::InvalidPacketSender` if the sender ID is invalid.
    /// - `NetError::InvalidPacketAddress` if the address is invalid.
    /// - `NetError::InvalidPacketPayload` if the payload is invalid.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` occurs if cannot toggle nonblocking mode or unknown error.
    /// - `NetError::Disconnected` if the connection is disconnected.
    #[allow(dead_code)]
    pub fn recv(&mut self) -> Result<Option<Packet>> {
        match self.raw.recv() {
            Ok(Some((client, mut packet))) => {
                if let Err(why) = self.validate(&client, &mut packet) {
                    if why == NetError::TooManyConnections
                        && self.valid_sender(packet.sender(), &client)
                    {
                        self.send_err(
                            packet.sender(),
                            ErrorPacket::TooManyConnections,
                            "Too many connections",
                        )?;
                        return Ok(None);
                    }

                    flee!(why);
                }

                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(why) => flee!(why),
        }
    }
}

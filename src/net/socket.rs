use std::mem;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::builtins::{ConnectionPayload, ErrorPayload, PingPayload};
use super::error::{ErrorPacket, NetError, Result};
use super::storage::{ClientStorage, StorageError};
use super::task::TaskScheduler;
use super::traits::{NetDecoder, SocketHandler};
use super::{
    ClientAddr, ClientId, Deliverable, LocalSocket, Packet, PacketLabel, RemoteSocket,
    SocketOptions,
};
use crate::net::error::InvalidPacketError;
use crate::{debugln, flee};

/// Default ID of the server.
const SERVER_CLIENT_ID: ClientId = ClientId(0);

/// Socket type for the connection. Either a remote or local connection.
enum SocketType {
    Remote(Box<RemoteSocket>), // Remote connection that uses UDP to communicate with a client / server.
    Local(LocalSocket),        // Local connection that uses MPSC to communicate locally.
}

impl SocketHandler for SocketType {
    #[inline]
    fn send(&self, dest: &ClientAddr, packet: Packet) -> Result<()> {
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
    id: ClientId,                    // Unique identifier for the connection.
    server_addr: Option<ClientAddr>, // The server address for the connection. Only set for clients.
    raw: SocketType,                 // Lower level socket type for the connection.

    clients: ClientStorage<ClientAddr>, // Storage for the clients connected to the socket.
    scheduler: TaskScheduler,           // Task scheduler for managing tasks.
}

impl Socket {
    /// Creates a new socket with the given socket type.
    fn new(socket: SocketType, opts: &SocketOptions, addr: Option<ClientAddr>) -> Result<Self> {
        let offset = ClientId(u16::from(opts.is_server()));
        let id = if opts.is_server() {
            SERVER_CLIENT_ID
        } else {
            ClientId::INVALID
        };

        let clients =
            match ClientStorage::new(offset, ClientId(opts.max_clients), ClientId::INVALID) {
                Ok(clients) => clients,
                Err(why) => flee!(NetError::StorageError(why.to_string())),
            };

        let mut socket = Self {
            id,
            server_addr: addr,
            raw: socket,

            clients,
            scheduler: TaskScheduler::new(opts.task_interval_ms),
        };

        if let Some(interval) = opts.archive_interval_ms {
            // Set the archive interval for clearing archived clients.
            socket.register_task("archive", interval, move |sock| {
                sock.clients.task_drain_archive(interval);
                Ok(())
            });
        }

        if let Some(interval) = opts.blacklist_interval_ms {
            // Set the blacklist interval for clearing blacklisted clients.
            socket.register_task("blacklist", interval, move |sock| {
                sock.clients.task_drain_blacklist(interval);
                Ok(())
            });
        }

        if let Some(interval) = opts.error_reset_interval_ms {
            // Set the error interval for clearing error counts.
            socket.register_task("error reset", interval, move |sock| {
                sock.clients.task_reset_errors(interval);
                Ok(())
            });
        }

        if let Some(interval) = opts.disconnect_interval_ms {
            // Register the disconnect task for expired clients.
            socket.register_task("expired", interval, move |sock| {
                for client_id in sock.expired_clients(interval) {
                    debugln!(
                        "[SERVER] Disconnecting client [{}] due to timeout.",
                        client_id
                    );

                    if sock.is_server() {
                        sock.disconnect_client(client_id, true)?;
                    } else {
                        flee!(NetError::Disconnected);
                    }
                }

                Ok(())
            });
        }

        if !socket.is_server() {
            if let Some(interval) = opts.ping_interval_ms {
                // Register the ping task.
                socket.register_task("ping", interval, |sock| {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                    let mut packet = Packet::new(PacketLabel::Ping, sock.id());
                    packet.set_payload(PingPayload(now, true));

                    sock.send(Deliverable {
                        to: ClientId(0),
                        packet,
                    })
                });
            }
        }

        Ok(socket)
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

        let server_opts = SocketOptions::default_server();
        let client_opts = SocketOptions::default_client();

        let server_addr = Some(ClientAddr::Local(SERVER_CLIENT_ID));
        Ok((
            Self::new(server, &server_opts, None)?,
            Self::new(client, &client_opts, server_addr)?,
        ))
    }

    /// Creates a new remote connection with the given address.
    pub fn new_remote(opts: &SocketOptions) -> Result<Self> {
        // Convert the server address from String to Client.
        let addr = if let Some(address) = &opts.server_address {
            match SocketAddr::from_str(address) {
                Ok(addr) => Some(ClientAddr::Ip(addr.ip(), addr.port())),
                Err(_) => flee!(NetError::SocketError(format!(
                    "Failed to parse server address: '{address}'. Please use a valid IP:PORT format.",
                ))),
            }
        } else {
            None
        };

        let socket = RemoteSocket::new(addr.is_none())?;
        Self::new(SocketType::Remote(Box::new(socket)), opts, addr)
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
    pub fn id(&self) -> ClientId {
        self.id
    }

    /// Returns clients that have not been active for a specified amount of time (in milliseconds).
    pub fn expired_clients(&self, timeout_ms: u64) -> Vec<ClientId> {
        self.clients.expired_clients(timeout_ms)
    }

    /// Obtains the UUIDs of the remote sockets.
    #[allow(dead_code)]
    #[inline]
    pub fn remote_ids(&self) -> Vec<ClientId> {
        self.clients.addr_iter().map(|(id, _)| id).collect()
    }

    /// Obtains the last sequence ID for the connection.
    #[allow(dead_code)]
    #[inline]
    pub fn last_sequence_id(&self, client_id: ClientId) -> Option<&u16> {
        self.clients.get_sequence(client_id)
    }

    /// Adds a new task to the scheduler.
    pub fn register_task<F, N: Into<String>>(&mut self, name: N, frequency_ms: u64, callback: F)
    where
        F: Fn(&mut Socket) -> Result<()> + Send + Sync + 'static,
    {
        self.scheduler.register(name, frequency_ms, callback);
    }

    /// Runs the tasks in the scheduler.
    pub fn run_tasks(&mut self, force: bool) -> Result<()> {
        if force || self.scheduler.is_ready() {
            let mut scheduler = mem::take(&mut self.scheduler);
            scheduler.run(self)?; // Run the tasks.
            self.scheduler = scheduler; // Move it back into `self`.
        }
        Ok(())
    }

    /// Adds a new client, returning the client's ID.
    fn add_client(&mut self, client: ClientAddr) -> Result<ClientId> {
        let (err, msg) = match self.clients.add(client) {
            Err(StorageError::AtCapacity) => (
                ErrorPacket::TooManyConnections,
                "Server is at maximum capacity for clients. Please try again later.",
            ),
            Err(StorageError::ClientExists) => (
                ErrorPacket::TooManyConnections,
                "Only one connection per IP allowed. Please try again later.",
            ),
            Err(StorageError::TimedOut) => (
                ErrorPacket::Blacklisted,
                "Your address is currently blacklisted. Please try again later.",
            ),
            Err(why) => flee!(NetError::StorageError(why.to_string())),
            Ok(client_id) => return Ok(client_id),
        };

        self.send_err(&client, err, msg)?;
        flee!(NetError::NothingToDo);
    }

    /// Queues a client for removal.
    fn queue_removal(&mut self, client_id: ClientId) {
        self.clients.archive_client(client_id);
    }

    /// Handles an invalid packet error. If there are too many errors, it will timeout the client.
    fn handle_invalid_packet_err(&mut self, error: &NetError) -> Result<()> {
        // Extract the address for invalid packets.
        let NetError::InvalidPacket(addr, ..) = error else {
            return Ok(());
        };

        // Handle the case where the socket is not in server mode or address in timeout.
        if !self.is_server() {
            return Ok(());
        } else if self.clients.is_blacklisted(addr) {
            flee!(NetError::NothingToDo);
        }

        self.clients.client_err(*addr);
        if let Some(errors) = self.clients.get_errors(addr) {
            if *errors > 5 {
                // Too many errors, disconnect the client.
                if let Some(client_id) = self.clients.get_id(addr) {
                    if let Err(why) = self.disconnect_client(client_id, false) {
                        debugln!("Failed to disconnect client with too many errors: {}", why);
                    }

                    self.clients.blacklist_client_addr(addr);
                } else {
                    // Client is not connected, but has too many errors.
                    self.clients.blacklist_client_addr(addr);
                }

                debugln!("Blacklisted client with too many errors: {}", addr);
                flee!(NetError::NothingToDo);
            }
        }

        Ok(())
    }

    /// Handles a packet that has invalid Client ID. Returns `true` if authenticated.
    ///
    /// # Errors
    ///
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::TooManyConnections` if the maximum number of clients has been reached.
    fn validate_invalid_client(&mut self, sender: &ClientAddr, packet: &mut Packet) -> Result<()> {
        // Check if a new client connecting, otherwise give it the old ID.
        if packet.label() == PacketLabel::Connect {
            // New client connecting, assign it a new ID.
            let cache_id = if self.is_remote() {
                // Remote connection, assign a new ID.
                self.add_client(*sender)?
            } else {
                // Need to generate an ID that is not INVALID_CLIENT_ID.
                let id = self.clients.next_id();
                self.add_client(ClientAddr::Local(id))?
            };

            packet.set_source(cache_id);
        } else if let Some(id) = self.clients.get_id(sender) {
            packet.set_source(id); // Discovered ID from cache.
        } else {
            // Client is not authenticated. Never sent connect packet.
            flee!(NetError::NotConnected(*sender));
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
    fn validate_client_lookup(&mut self, sender: &ClientAddr, client_id: ClientId) -> Result<()> {
        if let Some(cached_id) = self.clients.get_id(sender) {
            // Check that the client is using the correct ID.
            if client_id == cached_id {
                return Ok(());
            }

            // ID does not match address.
            flee!(NetError::InvalidPacket(
                *sender,
                InvalidPacketError::Source,
                format!(
                    "Client ID mismatch: expected {cached_id} but got {client_id} from address {sender}",
                )
            ));
        }

        // Only check the cache if address lookup failed.
        if let Some(cached) = self.clients.get_addr(client_id) {
            // Check that the ID is from the correct address.
            if sender == cached {
                return Ok(());
            }

            // Address does not match ID.
            flee!(NetError::InvalidPacket(
                *sender,
                InvalidPacketError::Source,
                format!(
                    "Client address mismatch: expected {cached} but got {sender} for ID {client_id}",
                )
            ));
        }

        // Socket Address and ID are not in the cache.
        flee!(NetError::NotConnected(*sender));
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
        if self.clients.is_blacklisted(sender) {
            flee!(NetError::NothingToDo);
        }

        let mut authed = !self.is_server();

        // Handles a packet with an invalid client ID.
        if packet.source() == ClientId::INVALID {
            self.validate_invalid_client(sender, packet)?;
            authed = true; // Would have error out if not authenticated.
        }

        // Check if the sender is in the cache.
        if self.is_server() && !authed {
            self.validate_client_lookup(sender, packet.source())?;
        }

        Ok(())
    }

    /// Processes the connection packet for the socket. This handles both server and client modes.
    fn packet_action_connection(&mut self, packet: &Packet, addr: &ClientAddr) -> Result<()> {
        let Ok((conn, _)) = ConnectionPayload::decode(packet.payload()) else {
            // Failed to decode connection payload, return an error.
            flee!(NetError::InvalidPacket(
                *addr,
                InvalidPacketError::Payload,
                "Could not parse connection payload".to_string()
            ));
        };

        if conn.0 != Packet::CURRENT_VERSION {
            flee!(NetError::InvalidPacket(
                *addr,
                InvalidPacketError::Version,
                format!(
                    "packet version mismatch {} != {}",
                    conn.0,
                    Packet::CURRENT_VERSION
                ),
            ));
        }

        if self.is_server() {
            // Server mode: Send connection payload to the client.
            let payload = ConnectionPayload(Packet::CURRENT_VERSION, packet.source(), 5000);
            let mut response = Packet::new(PacketLabel::Connect, self.id());
            response.set_payload(payload);
            self.send(Deliverable::new(packet.source(), response))?;
        } else {
            // Client mode: Accept the connection and set the ID.
            self.id = conn.1;
            self.clients.insert(packet.source(), *addr);
        }

        Ok(())
    }

    #[allow(clippy::unnecessary_wraps)]
    /// Processes a disconnection packet. This handles the removal of a client from the socket's storage.
    fn packet_action_disconnection(&mut self, packet: &Packet, _addr: &ClientAddr) -> Result<()> {
        // Remove the client from the storage.
        self.queue_removal(packet.source());
        Ok(())
    }

    /// Processes a ping packet. This handles both ping and pong packets.
    fn packet_action_ping(&mut self, packet: &Packet, addr: &ClientAddr) -> Result<()> {
        let Ok((ping, _)) = PingPayload::decode(packet.payload()) else {
            // Failed to decode ping payload, return an error.
            flee!(NetError::InvalidPacket(
                *addr,
                InvalidPacketError::Payload,
                "Could not parse ping payload".to_string()
            ));
        };

        if let Some(last) = self.clients.get_ping_mut(packet.source()) {
            *last = Instant::now();
        }

        if ping.1 {
            // Ping packet, send a pong packet back.
            let mut response = Packet::new(PacketLabel::Ping, self.id());
            response.set_payload(PingPayload(ping.0, false));
            self.send(Deliverable::new(packet.source(), response))?;
        }
        Ok(())
    }

    /// Processes the packet actions for errors. This handles the error packets and invokes the appropriate error handling.
    fn packet_actions_errors(&mut self, packet: &Packet, addr: &ClientAddr) -> Result<()> {
        if self.is_server() {
            return Ok(());
        }

        let Ok((payload, _)) = ErrorPayload::decode(packet.payload()) else {
            // Failed to decode error payload, return an error.
            // This means the payload was not a valid error packet.
            flee!(NetError::InvalidPacket(
                *addr,
                InvalidPacketError::Payload,
                "Could not parse error payload".to_string()
            ));
        };

        match payload.0 {
            ErrorPacket::TooManyConnections => {
                flee!(NetError::SocketError(
                    "Received 'TooManyConnections' error from server.".to_string()
                ));
            }
            ErrorPacket::Blacklisted => {
                flee!(NetError::SocketError(
                    "Received 'Blacklisted' error from server. You are temporarily blocked."
                        .to_string()
                ));
            }
            _ => {}
        }

        Ok(())
    }

    /// Handles the packet actions based on the packet type.
    fn packet_actions(&mut self, packet: &Packet, addr: &ClientAddr) -> Result<()> {
        let result = match packet.label() {
            PacketLabel::Connect => self.packet_action_connection(packet, addr),
            PacketLabel::Disconnect => self.packet_action_disconnection(packet, addr),
            PacketLabel::Ping => self.packet_action_ping(packet, addr),
            PacketLabel::Error => self.packet_actions_errors(packet, addr),
            _ => Ok(()),
        };

        // Handles the packet-related errors from the packet actions.
        if let Some(err) = result.err() {
            self.handle_invalid_packet_err(&err)?;
            flee!(err);
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
    pub fn disconnect_client(&mut self, client_id: ClientId, notify: bool) -> Result<()> {
        if !self.is_server() {
            flee!(NetError::NothingToDo);
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
    fn send_err(&mut self, to: &ClientAddr, error: ErrorPacket, msg: &str) -> Result<()> {
        let mut packet = Packet::new(PacketLabel::Error, self.id);
        let mut bytes = vec![error as u8];

        bytes.extend_from_slice(msg.as_bytes());
        packet.set_payload(bytes);

        // Attempt to set the Sequence ID.
        if let Some(client_id) = self.clients.get_id(to) {
            if let Some(seq) = self.clients.get_sequence_mut(client_id) {
                *seq = seq.wrapping_add(1);
                packet.set_sequence(*seq);
            }
        }

        self.raw.send(to, packet)
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
            debugln!(
                "Self connection detected: source ID {} and destination ID {}. Packet: {:?}.",
                self.id(),
                to,
                packet
            );
            flee!(NetError::NothingToDo);
        }

        // Update the sequence number for the packet if it's not a connect packet.
        if packet.source() != ClientId::INVALID || packet.label() != PacketLabel::Connect {
            if let Some(seq) = self.clients.get_sequence_mut(to) {
                *seq = seq.wrapping_add(1);
                packet.set_sequence(*seq);
            } else {
                flee!(NetError::NotConnected(ClientAddr::Local(to)));
            }
        }

        // Send the packet to the client.
        if let Some(client) = self.clients.get_addr(to) {
            self.raw.send(client, packet)
        } else if let Some(client) = self.server_addr() {
            self.raw.send(&client, packet)
        } else if !self.is_remote() {
            self.raw.send(&ClientAddr::Local(SERVER_CLIENT_ID), packet)
        } else {
            flee!(NetError::NotConnected(ClientAddr::Local(to)));
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
                    self.handle_invalid_packet_err(&why)?;
                    flee!(why);
                }

                self.packet_actions(&packet, &client)?;
                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(why) => {
                self.handle_invalid_packet_err(&why)?;
                flee!(why)
            }
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
                    self.handle_invalid_packet_err(&why)?;
                    flee!(why);
                }

                self.packet_actions(&packet, &client)?;
                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(why) => {
                self.handle_invalid_packet_err(&why)?;
                flee!(why)
            }
        }
    }
}

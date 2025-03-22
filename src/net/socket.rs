use crate::flee;

use super::{
    Deliverable, INVALID_CLIENT_ID, LocalSocket, NetError, Packet, PacketType, RemoteSocket, Result,
};

/// Socket type for the connection. Either a remote or local connection.
enum SocketType {
    Remote(Box<RemoteSocket>), // Remote connection that uses UDP to communicate with a client / server.
    Local(LocalSocket),        // Local connection that uses MPSC to communicate locally.
}

/// Trait for handling packets.
pub(crate) trait SocketHandler {
    /// Send a packet to the connection.
    #[allow(dead_code)]
    fn send(&mut self, deliverable: Deliverable) -> Result<()>;
    /// Try to receive a packet from the connection.
    #[allow(dead_code)]
    fn try_recv(&mut self) -> Result<Option<Packet>>;
    /// Waits to receive a packet from the connection.
    #[allow(dead_code)]
    fn recv(&mut self) -> Result<Option<Packet>>;
}

/// Socket for the connection. Used to send and receive packets to a client / server.
/// This is a unified interface for both local and remote connections.
pub struct Socket {
    socket: SocketType, // The socket type for the connection. Either a remote or local connection.
}

impl Socket {
    /// Invalid client ID, normally an uninitialized client.
    pub const INVALID_CLIENT_ID: u32 = INVALID_CLIENT_ID;

    /// Creates a new socket with the given socket type.
    fn new(socket: SocketType) -> Self {
        Self { socket }
    }

    /// Creates a new local connection pair.
    pub fn new_local_pair() -> Result<(Self, Self)> {
        let mut server_socket = LocalSocket::new(true);
        let mut client_socket = LocalSocket::new(false);

        // Obtain the receivers for both connections.
        let server_rx = server_socket.create_rx()?;
        let client_rx = client_socket.create_rx()?;

        // Set the receivers for both connections.
        server_socket.set_rx(client_rx)?;
        client_socket.set_rx(server_rx)?;

        let server = SocketType::Local(server_socket);
        let client = SocketType::Local(client_socket);
        Ok((Self::new(server), Self::new(client)))
    }

    /// Creates a new remote connection with the given address.
    pub fn new_remote(remote_address: Option<String>) -> Result<Self> {
        let socket = RemoteSocket::new(remote_address)?;
        Ok(Self::new(SocketType::Remote(Box::new(socket))))
    }

    /// Checks if the socket is a local connection.
    #[inline]
    pub fn is_local(&self) -> bool {
        match &self.socket {
            SocketType::Remote(_) => false,
            SocketType::Local(_) => true,
        }
    }

    /// Local address of the socket.
    #[inline]
    pub fn address(&self) -> &str {
        match &self.socket {
            SocketType::Remote(socket) => socket.address(),
            SocketType::Local(_) => LocalSocket::address(),
        }
    }

    /// Local ID of the socket.
    #[inline]
    pub fn id(&self) -> u32 {
        match &self.socket {
            SocketType::Remote(socket) => socket.id(),
            SocketType::Local(socket) => socket.id(),
        }
    }

    /// Checks if socket is in server mode.
    #[inline]
    pub fn is_server(&self) -> bool {
        match &self.socket {
            SocketType::Remote(socket) => socket.is_server(),
            SocketType::Local(socket) => socket.is_server(),
        }
    }

    /// Obtains the UUIDs of the remote sockets.
    #[inline]
    pub fn remote_ids(&self) -> Vec<u32> {
        match &self.socket {
            SocketType::Remote(socket) => socket.remote_ids(),
            SocketType::Local(socket) => socket.remote_ids(),
        }
    }

    /// Obtains the last sequence ID for the connection.
    #[allow(dead_code)]
    #[inline]
    pub fn last_sequence_id(&self, client_id: u32) -> Option<&u32> {
        match &self.socket {
            SocketType::Remote(socket) => socket.last_sequence_id(client_id),
            SocketType::Local(socket) => socket.last_sequence_id(client_id),
        }
    }

    /// Disconnects a client from the server and notifies the client if requested.
    ///
    /// # Errors
    ///
    /// - `NetError::NotServer` if the socket is not in server mode.
    /// - `NetError::SelfConnection` if the destination is the same as the source and the packet is not a connect packet.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` if there is a socket error.
    pub fn disconnect_client(&mut self, client_id: u32, notify: bool) -> Result<()> {
        if !self.is_server() {
            flee!(NetError::NotServer);
        }

        match &mut self.socket {
            SocketType::Remote(socket) => socket.disconnect_client(client_id, notify),
            SocketType::Local(socket) => socket.disconnect_client(client_id, notify),
        }
    }

    /// Sends a packet to the destination UUID. If the packet is a connect packet, it will not check for self connection.
    ///
    /// # Errors
    ///
    /// - `NetError::SelfConnection` if the destination is the same as the source and the packet is not a connect packet.
    /// - `NetError::NotConnected` if the connection is not established.
    /// - `NetError::SocketError` if there is a socket error.
    #[allow(dead_code)]
    pub fn send(&mut self, deliverable: Deliverable) -> Result<()> {
        if self.id() == deliverable.to && deliverable.packet.get_type() != PacketType::Connect {
            flee!(NetError::SelfConnection);
        }

        match &mut self.socket {
            SocketType::Remote(socket) => socket.send(deliverable),
            SocketType::Local(socket) => socket.send(deliverable),
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
        match &mut self.socket {
            SocketType::Remote(socket) => socket.try_recv(),
            SocketType::Local(socket) => socket.try_recv(),
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
        match &mut self.socket {
            SocketType::Remote(socket) => socket.recv(),
            SocketType::Local(socket) => socket.recv(),
        }
    }
}

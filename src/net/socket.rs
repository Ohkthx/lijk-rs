#![allow(dead_code)]

use anyhow::{Result, bail};
use uuid::Uuid;

use crate::net::{ConnectionError, PacketType};

use super::{Deliverable, LocalSocket, Packet, RemoteSocket};

/// Socket type for the connection. Either a remote or local connection.
enum SocketType {
    Remote(Box<RemoteSocket>), // Remote connection that uses UDP to communicate with a client / server.
    Local(LocalSocket),        // Local connection that uses MPSC to communicate locally.
}

/// Trait for handling packets.
pub(crate) trait SocketHandler {
    /// Send a packet to the connection.
    fn send(&mut self, deliverable: Deliverable) -> Result<()>;
    /// Try to receive a packet from the connection.
    fn try_recv(&mut self) -> Result<Option<Packet>>;
    /// Waits to receive a packet from the connection.
    fn recv(&mut self) -> Result<Packet>;
}

/// Socket for the connection. Used to send and receive packets to a client / server.
/// This is a unified interface for both local and remote connections.
pub struct Socket {
    socket: SocketType, // The socket type for the connection. Either a remote or local connection.
}

impl Socket {
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

    /// Local UUID of the socket.
    #[inline]
    pub fn uuid(&self) -> Uuid {
        match &self.socket {
            SocketType::Remote(socket) => socket.uuid(),
            SocketType::Local(socket) => socket.uuid(),
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
    pub fn remote_uuids(&self) -> Vec<Uuid> {
        match &self.socket {
            SocketType::Remote(socket) => socket.remote_uuids(),
            SocketType::Local(socket) => socket.remote_uuids(),
        }
    }

    /// Obtains the last sequence ID for the connection.
    #[inline]
    pub fn last_sequence_id(&self) -> u32 {
        match &self.socket {
            SocketType::Remote(socket) => socket.last_sequence_id(),
            SocketType::Local(socket) => socket.last_sequence_id(),
        }
    }

    /// Disconnects a client from the server and notifies the client if requested.
    pub fn disconnect_client(&mut self, uuid: Uuid, notify: bool) -> Result<()> {
        if !self.is_server() {
            bail!(ConnectionError::NotServer);
        }

        match &mut self.socket {
            SocketType::Remote(socket) => socket.disconnect_client(uuid, notify),
            SocketType::Local(socket) => socket.disconnect_client(uuid, notify),
        }
    }

    /// Sends a packet to the destination UUID. If the packet is a connect packet, it will not check for self connection.
    pub fn send(&mut self, deliverable: Deliverable) -> Result<()> {
        if self.uuid() == deliverable.to && deliverable.packet.get_type() != PacketType::Connect {
            bail!(ConnectionError::SelfConnection);
        }

        match &mut self.socket {
            SocketType::Remote(socket) => socket.send(deliverable),
            SocketType::Local(socket) => socket.send(deliverable),
        }
    }

    /// Tries to receive a packet from the connection. Returns None if no packet is available.
    pub fn try_recv(&mut self) -> Result<Option<Packet>> {
        match &mut self.socket {
            SocketType::Remote(socket) => socket.try_recv(),
            SocketType::Local(socket) => socket.try_recv(),
        }
    }

    /// Waits to receive a packet from the connection. Returns an error if a connection issue occurs.
    pub fn recv(&mut self) -> Result<Packet> {
        match &mut self.socket {
            SocketType::Remote(socket) => socket.recv(),
            SocketType::Local(socket) => socket.recv(),
        }
    }
}

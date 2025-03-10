mod local;
mod packet;

use anyhow::{Result, bail};
pub use local::LocalConnection;
pub use packet::{Packet, PacketType};

/// Error codes for various connection actions.
#[derive(Debug)]
pub enum ConnectionError {
    DuplicateConnection,        // Connection already exists.
    AuthenticationFailed,       // Authentication failed.
    NotConnected,               // Connection does not exist.
    Disconnected,               // Connection is disconnected.
    InvalidPacketVersion(u8),   // Packet version is invalid.
    InvalidPacketLength(usize), // Packet length is invalid.
}

impl std::error::Error for ConnectionError {}
impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::Disconnected => write!(f, "Disconnected"),
            ConnectionError::AuthenticationFailed => write!(f, "Authentication Failed"),
            ConnectionError::NotConnected => write!(f, "Not Connected"),
            ConnectionError::DuplicateConnection => write!(f, "Duplicate Connection"),
            ConnectionError::InvalidPacketVersion(version) => write!(
                f,
                "Invalid Packet Version, expected {}, got {}",
                Packet::VERSION,
                version
            ),
            ConnectionError::InvalidPacketLength(size) => write!(
                f,
                "Invalid Packet Length, expected {}, got {}",
                Packet::HEADER_SIZE,
                size
            ),
        }
    }
}

/// Trait for handling packets.
pub trait PacketHandler {
    /// Send a packet to the connection.
    fn send(&self, packet: Packet);
    /// Try to receive a packet from the connection.
    fn try_recv(&self) -> Result<Option<Packet>>;
    /// Waits to receive a packet from the connection.
    fn recv(&self) -> Result<Packet>;
}

/// Connection used to communicate between clients and servers.
pub enum Connection {
    /// Local connection that uses MPSC to communicate locally.
    Local(LocalConnection),
    /// Remote connection that uses a network connection to communicate.
    #[allow(dead_code)]
    Remote,
}

impl Connection {
    /// Creates a new local connection. Returning a tuple of two connections.
    pub fn new_local() -> Result<(Self, Self)> {
        let mut server = LocalConnection::new();
        let mut client = LocalConnection::new();

        // Obtain the receivers for both connections.
        let server_rx = server.create_rx()?;
        let client_rx = client.create_rx()?;

        // Set the receivers for both connections.
        server.set_rx(client_rx)?;
        client.set_rx(server_rx)?;

        Ok((Self::Local(server), Self::Local(client)))
    }
}

impl PacketHandler for Connection {
    fn send(&self, packet: Packet) {
        if let Self::Local(connection) = self {
            connection.send(packet);
        }
    }

    fn try_recv(&self) -> Result<Option<Packet>> {
        if let Self::Local(connection) = self {
            connection.try_recv()
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }

    fn recv(&self) -> Result<Packet> {
        if let Self::Local(connection) = self {
            connection.recv()
        } else {
            bail!(ConnectionError::NotConnected);
        }
    }
}

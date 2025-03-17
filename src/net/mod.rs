mod local;
mod packet;
mod remote;
mod socket;

use uuid::Uuid;

pub(crate) use local::LocalSocket;
pub(crate) use remote::RemoteSocket;

pub use packet::{Packet, PacketType};
pub use socket::Socket;

/// Used to specify the destination and packet for a socket action.
pub struct Deliverable {
    pub(crate) to: Uuid,
    /// Destination UUID for the packet.
    pub(crate) packet: Packet, // Packet to be sent to the destination.
}

impl Deliverable {
    /// Creates a new deliverable with the given destination and packet.
    pub fn new(to: Uuid, packet: Packet) -> Self {
        Self { to, packet }
    }
}

/// Error codes for various connection actions.
#[derive(Debug)]
pub enum ConnectionError {
    DuplicateConnection,                  // Connection already exists.
    AuthenticationFailed,                 // Authentication failed.
    NotServer,                            // Connection is not a server.
    NotConnected,                         // Connection does not exist.
    Disconnected,                         // Connection is disconnected.
    Timeout,                              // Connection timed out.
    SelfConnection,                       // Connection to self is not allowed.
    InvalidPacketUuid(Uuid, Uuid),        // Packet UUID is invalid.
    InvalidPacketAddress(String, String), // Packet address is invalid.
    InvalidPacketVersion(u8),             // Packet version is invalid.
    InvalidPacketLength(usize),           // Packet length is invalid.
    InvalidPacketPayload(String),         // Packet payload is invalid.
    SocketError(String),                  // Socket error occurred.
}

impl std::error::Error for ConnectionError {}
impl std::fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionError::DuplicateConnection => write!(f, "Duplicate Connection"),
            ConnectionError::AuthenticationFailed => write!(f, "Authentication Failed"),
            ConnectionError::NotServer => write!(f, "Socket is not a Server"),
            ConnectionError::NotConnected => write!(f, "Not Connected"),
            ConnectionError::Disconnected => write!(f, "Disconnected"),
            ConnectionError::Timeout => write!(f, "Connection Timeout"),
            ConnectionError::SelfConnection => write!(f, "Self Connection is not allowed"),
            ConnectionError::InvalidPacketUuid(expected, got) => {
                write!(f, "Invalid Packet UUID, expected {expected}, got {got}")
            }
            ConnectionError::InvalidPacketAddress(expected, got) => {
                write!(f, "Invalid Packet Address, expected {expected}, got {got}")
            }
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
            ConnectionError::InvalidPacketPayload(expected) => {
                write!(f, "Invalid Packet Payload, expected {expected}")
            }
            ConnectionError::SocketError(why) => write!(f, "Socket Error: {why}"),
        }
    }
}

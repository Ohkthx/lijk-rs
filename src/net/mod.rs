mod local;
mod packet;
mod remote;
mod socket;
mod storage;

pub(crate) use local::LocalSocket;
pub(crate) use remote::RemoteSocket;

pub use packet::{Packet, PacketError, PacketType};
pub use socket::Socket;
use storage::ClientStorage;

/// ID for the server.
pub(crate) const SERVER_ID: u32 = 0;
/// Invalid ID for a client.
pub(crate) const INVALID_CLIENT_ID: u32 = ClientStorage::<()>::INVALID_CLIENT_ID;

/// Used to specify the destination and packet for a socket action.
pub struct Deliverable {
    pub(crate) to: u32,        // ID of the destination user.
    pub(crate) packet: Packet, // Packet to be sent to the destination.
}

impl Deliverable {
    /// Creates a new deliverable with the given destination and packet.
    pub fn new(to: u32, packet: Packet) -> Self {
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
    TooManyConnections,                   // Too many connections.
    InvalidPacketSender(u32, u32),        // Packet Sender ID is invalid.
    InvalidPacketAddress(String, String), // Packet address is invalid.
    InvalidPacketVersion(u8),             // Packet version is invalid.
    InvalidPacketPayload(String),         // Packet payload is invalid.
    InvalidPacket(usize, usize, String),  // Packet is invalid.
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
            ConnectionError::TooManyConnections => write!(f, "Too Many Connections"),
            ConnectionError::InvalidPacketSender(expected, got) => {
                write!(
                    f,
                    "Invalid Packet Sender ID, expected {expected}, got {got}"
                )
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
            ConnectionError::InvalidPacketPayload(expected) => {
                write!(f, "Invalid Packet Payload, expected {expected}")
            }
            ConnectionError::InvalidPacket(expected, got, reason) => {
                write!(
                    f,
                    "Invalid Packet, minimum size {expected}, got {got}, reason: {reason}"
                )
            }
            ConnectionError::SocketError(why) => write!(f, "Socket Error: {why}"),
        }
    }
}

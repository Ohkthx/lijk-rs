use super::{ClientAddr, EntityId};

/// Result type for network actions.
pub(crate) type Result<T> = std::result::Result<T, NetError>;

/// Error codes included in the `PacketLabel::Error` packet.
#[derive(Debug, PartialEq)]
pub enum ErrorPacket {
    TooManyConnections = 0x01, // Too many connections.
    InvalidPacketVersion,      // Invalid packet version.
    InvalidPacketSize,         // Invalid packet size.
    InvalidPacketLabel,        // Invalid packet label.
}

impl std::fmt::Display for ErrorPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorPacket::TooManyConnections => write!(f, "Too many connections"),
            ErrorPacket::InvalidPacketVersion => write!(f, "Invalid packet version"),
            ErrorPacket::InvalidPacketSize => write!(f, "Invalid packet size"),
            ErrorPacket::InvalidPacketLabel => write!(f, "Invalid packet label"),
        }
    }
}

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum NetError {
    DuplicateConnection,                              // Connection already exists.
    NotServer,                                        // Connection is not a server.
    NotConnected(ClientAddr, bool),                   // Non-existing connection.
    Disconnected,                                     // Connection is disconnected.
    Timeout,                                          // Connection timed out.
    SelfConnection,                                   // Connection to self is not allowed.
    TooManyConnections,                               // Too many connections.
    StorageError(String),                             // Error in storage.
    InvalidServerAddress(String),                     // Server is invalid.
    SocketError(String),                              // Socket error occurred.
    InvalidPacketSender(EntityId, EntityId),          // Packet Sender ID is invalid.
    InvalidPacketAddress(String, String),             // Packet address is invalid.
    InvalidPacketPayload(String),                     // Packet payload is invalid.
    InvalidPacket(ErrorPacket, Option<usize>, usize), // Packet is invalid.
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::DuplicateConnection => write!(f, "duplicate connection"),
            NetError::NotServer => write!(f, "socket is not a server"),
            NetError::Disconnected => write!(f, "disconnected"),
            NetError::Timeout => write!(f, "connection timeout"),
            NetError::SelfConnection => write!(f, "self connection is not allowed"),
            NetError::TooManyConnections => write!(f, "too many connections"),
            NetError::StorageError(why) => write!(f, "storage experienced {why}"),
            NetError::InvalidServerAddress(addr) => write!(f, "invalid server address: {addr}"),
            NetError::SocketError(why) => write!(f, "socket error: {why}"),
            NetError::NotConnected(client, is_remote) => {
                if *is_remote {
                    write!(f, "not connected to destination {client}")
                } else {
                    write!(f, "not connected from {client}")
                }
            }
            NetError::InvalidPacketSender(expected, got) => {
                write!(
                    f,
                    "invalid packet sender ID, expected {expected}, got {got}"
                )
            }
            NetError::InvalidPacketAddress(expected, got) => {
                write!(f, "invalid packet address, expected {expected}, got {got}")
            }
            NetError::InvalidPacketPayload(expected) => {
                write!(f, "invalid packet payload, expected {expected}")
            }
            NetError::InvalidPacket(error, expected, got) => {
                if let Some(expected) = expected {
                    write!(
                        f,
                        "invalid packet, expected {expected}, got {got}, reason: {error}"
                    )
                } else {
                    write!(f, "invalid packet, got {got}, reason: {error}")
                }
            }
        }
    }
}

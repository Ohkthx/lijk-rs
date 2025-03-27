use super::{ClientAddr, EntityId};

/// Result type for network actions.
pub(crate) type Result<T> = std::result::Result<T, NetError>;

/// Error codes included in the `PacketLabel::Error` packet.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum ErrorPacket {
    TooManyConnections = 0x01, // Too many connections.
    Blacklisted,               // Connection is blacklisted.
    InvalidPacketVersion,      // Invalid packet version.
    InvalidPacketSize,         // Invalid packet size.
    InvalidPacketLabel,        // Invalid packet label.
    Unknown,                   // Unknown error.
}
impl From<ErrorPacket> for u8 {
    fn from(label: ErrorPacket) -> Self {
        label as u8
    }
}

impl From<u8> for ErrorPacket {
    fn from(value: u8) -> Self {
        match value {
            0x01 => ErrorPacket::TooManyConnections,
            0x02 => ErrorPacket::Blacklisted,
            0x03 => ErrorPacket::InvalidPacketVersion,
            0x04 => ErrorPacket::InvalidPacketSize,
            0x05 => ErrorPacket::InvalidPacketLabel,
            _ => ErrorPacket::Unknown,
        }
    }
}

impl std::fmt::Display for ErrorPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorPacket::TooManyConnections => write!(f, "Too many connections"),
            ErrorPacket::Blacklisted => write!(f, "Connection is blacklisted"),
            ErrorPacket::InvalidPacketVersion => write!(f, "Invalid packet version"),
            ErrorPacket::InvalidPacketSize => write!(f, "Invalid packet size"),
            ErrorPacket::InvalidPacketLabel => write!(f, "Invalid packet label"),
            ErrorPacket::Unknown => write!(f, "Unknown error"),
        }
    }
}

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum NetError {
    NothingToDo,                                                  // No action needed.
    DuplicateConnection,                                          // Connection already exists.
    NotServer,                                                    // Connection is not a server.
    NotConnected(ClientAddr, bool),                               // Non-existing connection.
    Disconnected,                                                 // Connection is disconnected.
    Timeout,                                                      // Connection timed out.
    SelfConnection,               // Connection to self is not allowed.
    TooManyConnections,           // Too many connections.
    StorageError(String),         // Error in storage.
    InvalidServerAddress(String), // Server is invalid.
    SocketError(String),          // Socket error occurred.
    InvalidPacketSender(ClientAddr, EntityId, EntityId), // Packet Sender ID is invalid.
    InvalidPacketAddress(ClientAddr, String, String), // Packet address is invalid.
    InvalidPacketPayload(ClientAddr, String), // Packet payload is invalid.
    InvalidPacketParse(ErrorPacket, Option<usize>, usize), // Packet parsing error.
    InvalidPacket(ClientAddr, ErrorPacket, Option<usize>, usize), // Packet is invalid.
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::NothingToDo => write!(f, "nothing to do"),
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
            NetError::InvalidPacketSender(addr, expected, got) => {
                write!(
                    f,
                    "invalid packet sender ID from {addr}, expected {expected}, got {got}"
                )
            }
            NetError::InvalidPacketAddress(addr, expected, got) => {
                write!(
                    f,
                    "invalid packet address from {addr}, expected {expected}, got {got}"
                )
            }
            NetError::InvalidPacketPayload(addr, expected) => {
                write!(f, "invalid packet payload from {addr}, expected {expected}")
            }
            NetError::InvalidPacketParse(error, expected, got) => {
                if let Some(expected) = expected {
                    write!(
                        f,
                        "invalid packet parse, expected {expected}, got {got}, reason: {error}"
                    )
                } else {
                    write!(f, "invalid packet parse, got {got}, reason: {error}")
                }
            }
            NetError::InvalidPacket(addr, error, expected, got) => {
                if let Some(expected) = expected {
                    write!(
                        f,
                        "invalid packet from {addr}, expected {expected}, got {got}, reason: {error}"
                    )
                } else {
                    write!(f, "invalid packet from {addr}, got {got}, reason: {error}")
                }
            }
        }
    }
}

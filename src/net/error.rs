use super::ClientAddr;
use super::netcode_derive::{NetDecode, NetEncode};
use super::traits::{NetDecoder, NetEncoder};

/// Result type for network actions.
pub(crate) type Result<T> = std::result::Result<T, NetError>;

/// Error codes included in the `PacketLabel::Error` packet.
#[derive(Debug, PartialEq, Copy, Clone, NetEncode, NetDecode)]
pub enum ErrorPacket {
    TooManyConnections,   // Too many connections.
    Blacklisted,          // Connection is blacklisted.
    InvalidPacketVersion, // Invalid packet version.
    Unknown,              // Unknown error.
}

impl std::fmt::Display for ErrorPacket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorPacket::TooManyConnections => write!(f, "Too many connections"),
            ErrorPacket::Blacklisted => write!(f, "Connection is blacklisted"),
            ErrorPacket::InvalidPacketVersion => write!(f, "Invalid packet version"),
            ErrorPacket::Unknown => write!(f, "Unknown error"),
        }
    }
}

/// Represents errors that can occur when processing packets.
#[derive(Debug, PartialEq, Eq)]
pub enum InvalidPacketError {
    Header,  // The packet header is invalid or malformed. This usually indicates a decoding error.
    Version, // The packet version is invalid or unsupported.
    Source,  // The source of the packet is invalid, ClientId or Address.
    Payload, // The payload of the packet is invalid or cannot be decoded.
}

impl std::fmt::Display for InvalidPacketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvalidPacketError::Header => write!(f, "Invalid packet header"),
            InvalidPacketError::Version => write!(f, "Invalid packet version"),
            InvalidPacketError::Source => write!(f, "Invalid packet source"),
            InvalidPacketError::Payload => write!(f, "Invalid packet payload"),
        }
    }
}

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum NetError {
    NothingToDo, // No action needed.

    // Status errors.
    NotConnected(ClientAddr), // Not connected to `ClientAddr`.
    Disconnected,             // Connection is disconnected.
    SocketError(String),      // Socket error occurred. Unrecoverable.

    // Storage errors.
    StorageError(String), // Error in storage.

    // Packet errors.
    NetCode(String),                                       // Network code error.
    InvalidPacket(ClientAddr, InvalidPacketError, String), // Packet is invalid.
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::NothingToDo => write!(f, "nothing to do"),
            NetError::Disconnected => write!(f, "disconnected from the connection"),
            NetError::StorageError(why) => write!(f, "storage experienced {why}"),
            NetError::SocketError(why) => write!(f, "socket error: {why}"),
            NetError::NetCode(why) => write!(f, "network code error: {why}"),
            NetError::NotConnected(client) => write!(f, "not connected to destination {client}"),
            NetError::InvalidPacket(addr, error, why) => {
                write!(f, "invalid packet from {addr}, reason: {error}: {why}")
            }
        }
    }
}

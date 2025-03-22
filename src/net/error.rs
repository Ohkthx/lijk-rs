/// Result type for network actions.
pub(crate) type Result<T> = std::result::Result<T, NetError>;

/// Error codes included in the `PacketType::Error` packet.
#[derive(Debug)]
pub enum ErrorPacket {
    TooManyConnections = 0x01, // Too many connections.
    InvalidPacketVersion,      // Invalid packet version.
    InvalidPacketSize,         // Invalid packet size.
    InvalidPacketType,         // Invalid packet type.
}

/// Error codes for various connection actions.
#[derive(Debug, PartialEq)]
pub enum NetError {
    DuplicateConnection,                         // Connection already exists.
    NotServer,                                   // Connection is not a server.
    NotConnected(bool),                          // Non-existing connection, bool: is_server.
    Disconnected,                                // Connection is disconnected.
    Timeout,                                     // Connection timed out.
    SelfConnection,                              // Connection to self is not allowed.
    TooManyConnections,                          // Too many connections.
    InvalidPacketSender(u32, u32),               // Packet Sender ID is invalid.
    InvalidPacketAddress(String, String),        // Packet address is invalid.
    InvalidPacketPayload(String),                // Packet payload is invalid.
    InvalidPacket(Option<usize>, usize, String), // Packet is invalid.
    InvalidPacketVersion(u8, u8),                // Packet version is invalid.
    InvalidPacketSize(usize, usize),             // Packet header is invalid.
    InvalidPacketType(u8),                       // Packet type is invalid.
    SocketError(String),                         // Socket error occurred.
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::DuplicateConnection => write!(f, "Duplicate Connection"),
            NetError::NotServer => write!(f, "Socket is not a Server"),
            NetError::Disconnected => write!(f, "Disconnected"),
            NetError::Timeout => write!(f, "Connection Timeout"),
            NetError::SelfConnection => write!(f, "Self Connection is not allowed"),
            NetError::TooManyConnections => write!(f, "Too Many Connections"),
            NetError::NotConnected(is_server) => {
                if *is_server {
                    write!(f, "Not connect to server.")
                } else {
                    write!(f, "Client is not connected / authenticated.")
                }
            }
            NetError::InvalidPacketSender(expected, got) => {
                write!(
                    f,
                    "Invalid Packet Sender ID, expected {expected}, got {got}"
                )
            }
            NetError::InvalidPacketAddress(expected, got) => {
                write!(f, "Invalid Packet Address, expected {expected}, got {got}")
            }
            NetError::InvalidPacketPayload(expected) => {
                write!(f, "Invalid Packet Payload, expected {expected}")
            }
            NetError::InvalidPacket(expected, got, reason) => {
                if let Some(expected) = expected {
                    write!(
                        f,
                        "Invalid Packet, expected {expected}, got {got}, reason: {reason}"
                    )
                } else {
                    write!(f, "Invalid Packet, got {got}, reason: {reason}")
                }
            }
            NetError::InvalidPacketVersion(expected, got) => {
                write!(f, "Invalid Packet Version, expected {expected}, got {got}")
            }
            NetError::InvalidPacketSize(expected, got) => {
                write!(
                    f,
                    "Invalid Packet Size, minimum size expected {expected}, got {got}"
                )
            }
            NetError::InvalidPacketType(got) => write!(f, "Invalid Packet Type: {got}"),
            NetError::SocketError(why) => write!(f, "Socket Error: {why}"),
        }
    }
}

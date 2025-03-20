use anyhow::{Result, bail};

/// Packet types for connections that can be sent.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum PacketType {
    Error = 0x00, // Error packet, used to specify an error.
    Acknowledge,  // Acknowledge an action.
    Connect,      // Connect to a server or client.
    Disconnect,   // Disconnect from a server or client.
    Heartbeat,    // Heartbeat packet, used to check if the connection is alive.
    Message,      // Message packet, used to send a message to a server or client.
}

impl TryFrom<u8> for PacketType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self> {
        let packet_type = match value {
            0x00 => PacketType::Error,
            0x01 => PacketType::Acknowledge,
            0x02 => PacketType::Connect,
            0x03 => PacketType::Disconnect,
            0x04 => PacketType::Heartbeat,
            0x05 => PacketType::Message,
            _ => bail!("Unknown packet type: {}.", value),
        };

        Ok(packet_type)
    }
}

/// Error codes included in the `PacketType::Error` packet.
#[derive(Debug)]
pub enum PacketError {
    TooManyConnections = 0x01, // Too many connections.
}

/// A packet that be sent over a connection.
#[derive(Debug, Clone)]
pub struct Packet {
    version: u8,        // Current version of the packet.
    r#type: PacketType, // Type of the packet, used to specify the type of action.
    sender: u32,        // ID of the sender.
    sequence: u32,      // Sequence number for ordering packets.
    payload: Vec<u8>,   // Extra payload / data to be sent.
}

impl Packet {
    /// Current version of Packets.
    pub(crate) const VERSION: u8 = 0x01;
    /// Minimum size of a packet.
    pub(crate) const HEADER_SIZE: usize =
        size_of::<u8>() + size_of::<u8>() + size_of::<u32>() + size_of::<u32>();

    /// Creates a new packet with the given type and sender UUID.
    #[inline]
    pub fn new(packet_type: PacketType, sender: u32) -> Self {
        Self {
            version: Self::VERSION,
            r#type: packet_type,
            sender,
            sequence: 0,
            payload: Vec::new(),
        }
    }

    /// Obtains the version of the packet.
    #[inline]
    pub fn get_version(&self) -> u8 {
        self.version
    }

    /// Obtains the type.
    #[inline]
    pub fn get_type(&self) -> PacketType {
        self.r#type
    }

    /// Obtains the Sender's ID.
    #[inline]
    pub fn get_sender(&self) -> u32 {
        self.sender
    }

    /// Sets the Sender's ID for the packet.
    #[inline]
    pub fn set_sender(&mut self, id: u32) {
        self.sender = id;
    }

    /// Obtains the sequencing number for packet ordering.
    #[inline]
    pub fn get_sequence(&self) -> u32 {
        self.sequence
    }

    /// Sets the sequence number of the packet.
    #[inline]
    pub fn set_sequence(&mut self, sequence: u32) {
        self.sequence = sequence;
    }

    /// Obtains the payload of the packet.
    #[inline]
    pub fn get_payload(&self) -> &[u8] {
        &self.payload
    }

    /// Sets the payload of the packet.
    #[inline]
    pub fn set_payload<T: Into<Vec<u8>>>(&mut self, payload: T) {
        self.payload = payload.into();
    }
}

impl From<&Packet> for Vec<u8> {
    fn from(packet: &Packet) -> Self {
        let mut buffer = vec![0; Packet::HEADER_SIZE + packet.payload.len()];
        buffer[0] = packet.version;
        buffer[1] = packet.r#type as u8;
        buffer[2..6].copy_from_slice(&packet.sender.to_be_bytes());
        buffer[6..10].copy_from_slice(&packet.sequence.to_be_bytes());
        buffer[Packet::HEADER_SIZE..].copy_from_slice(&packet.payload);
        buffer
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::HEADER_SIZE {
            bail!("Packet is too short to include the full header.");
        }

        // Parse the header.
        let version = bytes[0];
        let packet_type = PacketType::try_from(bytes[1])?;
        let sender = u32::from_be_bytes(bytes[2..6].try_into().unwrap());
        let sequence = u32::from_be_bytes(bytes[6..10].try_into().unwrap());

        // Remainder is payload.
        let payload = bytes[Self::HEADER_SIZE..].to_vec();

        Ok(Self {
            version,
            r#type: packet_type,
            sender,
            sequence,
            payload,
        })
    }
}

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

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => PacketType::Error,
            0x01 => PacketType::Acknowledge,
            0x02 => PacketType::Connect,
            0x03 => PacketType::Disconnect,
            0x04 => PacketType::Heartbeat,
            0x05 => PacketType::Message,
            _ => panic!("Invalid packet type"),
        }
    }
}

/// Error codes included in the `PacketType::Error` packet.
pub enum PacketError {
    TooManyConnections = 0x01, // Too many connections.
}

/// A raw packet that can be sent over a connection.
pub(crate) struct RawPacket {
    data: Vec<u8>, // Raw data of the packet, including the header and payload.
}

impl RawPacket {
    /// Validates the length of the raw packet to ensure it is at least the size of the header.
    #[inline]
    pub(crate) fn is_valid_len(&self) -> bool {
        self.data.len() >= Packet::HEADER_SIZE
    }

    /// Obtains the underlying data of the raw packet.
    #[inline]
    pub(crate) fn get_data(&self) -> &[u8] {
        &self.data
    }
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
    pub fn set_payload(&mut self, payload: &[u8]) {
        self.payload = payload.to_vec();
    }

    /// Converts the packet into Vec<u8> for sending.
    #[inline]
    pub fn as_bytes(&self) -> Vec<u8> {
        RawPacket::from(self).into()
    }
}

impl From<Packet> for RawPacket {
    fn from(packet: Packet) -> Self {
        RawPacket::from(&packet)
    }
}

impl From<&Packet> for RawPacket {
    fn from(packet: &Packet) -> Self {
        let mut raw = vec![0; Packet::HEADER_SIZE + packet.get_payload().len()];
        raw[0] = packet.get_version();
        raw[1] = packet.get_type() as u8;
        raw[2..6].copy_from_slice(&packet.get_sender().to_le_bytes());
        raw[6..10].copy_from_slice(&packet.get_sequence().to_le_bytes());
        raw[Packet::HEADER_SIZE..].copy_from_slice(packet.get_payload());

        Self { data: raw }
    }
}

impl From<RawPacket> for Packet {
    fn from(value: RawPacket) -> Self {
        let version = value.data[0];
        let packet_type = PacketType::from(value.data[1]);
        let sender = u32::from_le_bytes(value.data[2..6].try_into().expect("Invalid sender ID"));
        let sequence = u32::from_le_bytes(value.data[6..10].try_into().expect("Invalid sequence"));

        let payload = if value.is_valid_len() {
            value.data[Self::HEADER_SIZE..].to_vec()
        } else {
            vec![]
        };

        Self {
            version,
            r#type: packet_type,
            sender,
            sequence,
            payload,
        }
    }
}

impl From<RawPacket> for Vec<u8> {
    fn from(value: RawPacket) -> Self {
        value.data
    }
}

impl From<&[u8]> for RawPacket {
    fn from(value: &[u8]) -> Self {
        Self {
            data: value.to_vec(),
        }
    }
}

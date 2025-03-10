use uuid::Uuid;

/// Packet types for connections that can be sent.
pub enum PacketType {
    Error = 0x00,
    Acknowledge,
    Connect,
    Disconnect,
    Ping,
    Pong,
    Message,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => PacketType::Error,
            0x01 => PacketType::Acknowledge,
            0x02 => PacketType::Connect,
            0x03 => PacketType::Disconnect,
            0x04 => PacketType::Ping,
            0x05 => PacketType::Pong,
            0x06 => PacketType::Message,
            _ => panic!("Invalid packet type"),
        }
    }
}

/// A packet that be sent over a connection.
pub struct Packet {
    data: Vec<u8>,
}

impl Packet {
    /// Current version of the packet.
    pub(crate) const VERSION: u8 = 0x01;
    /// Base size of the packet header.
    pub(crate) const HEADER_SIZE: usize = size_of::<u8>() + size_of::<u8>() + size_of::<Uuid>();

    /// Creates a new packet with the given type and UUID.
    #[inline]
    pub fn new(r#type: PacketType, uuid: Uuid) -> Self {
        let mut packet = Self {
            data: vec![0; Self::HEADER_SIZE],
        };

        packet.set_version();
        packet.set_type(r#type);
        packet.set_uuid(uuid);
        packet
    }

    /// Obtains the version of the packet.
    #[inline]
    pub fn get_version(&self) -> u8 {
        self.data[0]
    }

    /// Obtains the type of the packet.
    #[inline]
    pub fn get_type(&self) -> PacketType {
        self.data[1].into()
    }

    /// Obtains the UUID of the packet.
    #[inline]
    pub fn get_uuid(&self) -> Uuid {
        let uuid_bytes = &self.data[2..18];
        Uuid::from_slice(uuid_bytes).expect("Invalid UUID")
    }

    /// Obtains the length of the packet.
    #[inline]
    pub fn get_length(&self) -> usize {
        self.data.len()
    }

    /// Obtains the payload of the packet.
    #[inline]
    pub fn get_payload(&self) -> &[u8] {
        &self.data[Self::HEADER_SIZE..]
    }

    /// Checks if the packet is of the minimum size.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.data.len() >= Self::HEADER_SIZE
    }

    /// Sets the version of the packet.
    #[inline]
    fn set_version(&mut self) {
        self.data[0] = Self::VERSION;
    }

    /// Sets the type of the packet.
    #[inline]
    fn set_type(&mut self, r#type: PacketType) {
        self.data[1] = r#type as u8;
    }

    /// Sets the UUID of the packet.
    #[inline]
    fn set_uuid(&mut self, uuid: Uuid) {
        let uuid_bytes = uuid.as_bytes();
        self.data[2..18].copy_from_slice(uuid_bytes);
    }

    /// Sets the payload of the packet.
    #[inline]
    pub fn set_payload(&mut self, payload: &[u8]) {
        let payload_size = payload.len();
        let total_size = Self::HEADER_SIZE + payload_size;

        self.data.resize(total_size, 0);
        self.data[Self::HEADER_SIZE..].copy_from_slice(payload);
    }
}

impl AsRef<[u8]> for Packet {
    fn as_ref(&self) -> &[u8] {
        &self.data
    }
}

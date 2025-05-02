use super::ClientId;
use super::error::{NetError, Result};
use super::netcode_derive::{NetDecode, NetEncode};
use super::traits::{NetDecoder, NetEncoder};

/// Packet labels for connections that can be sent.
#[derive(PartialEq, Copy, Clone, Debug)]
#[repr(u8)]
pub enum PacketLabel {
    /// Error packet, used to specify an error.
    Error = 0x00,
    /// Acknowledge an action.
    Acknowledge,
    /// Connect to a server or client.
    Connect,
    /// Disconnect from a server or client.
    Disconnect,
    /// Ping packet, used to check if the connection is alive.
    Ping,
    /// Message packet, used to send a message to a server or client.
    Message,
    /// Expandable packet label, can be >= 0x06.
    Extension(u8),
}

impl NetEncoder for PacketLabel {
    fn encode(self) -> Vec<u8> {
        // Encode the packet label as a single byte.
        let mut buffer = vec![0; 1];
        buffer[0] = match self {
            PacketLabel::Error => 0x00,
            PacketLabel::Acknowledge => 0x01,
            PacketLabel::Connect => 0x02,
            PacketLabel::Disconnect => 0x03,
            PacketLabel::Ping => 0x04,
            PacketLabel::Message => 0x05,
            PacketLabel::Extension(value) => value,
        };
        buffer
    }
}

impl NetDecoder for PacketLabel {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(NetError::NetCode(
                "PacketLabel::decode: data is empty".to_string(),
            ));
        }

        match data[0] {
            0x00 => Ok((PacketLabel::Error, 1)),
            0x01 => Ok((PacketLabel::Acknowledge, 1)),
            0x02 => Ok((PacketLabel::Connect, 1)),
            0x03 => Ok((PacketLabel::Disconnect, 1)),
            0x04 => Ok((PacketLabel::Ping, 1)),
            0x05 => Ok((PacketLabel::Message, 1)),
            value => Ok((PacketLabel::Extension(value), 1)),
        }
    }
}

/// A packet that be sent over a connection.
#[derive(Debug, Clone, NetEncode, NetDecode)]
pub struct Packet {
    label: PacketLabel, // Label of the packet.
    source: ClientId,   // ID of the source.
    sequence: u16,      // Sequence number for ordering packets.
    payload: Vec<u8>,   // Extra payload / data to be sent.
}

impl Packet {
    /// Current version of Packets.
    pub(crate) const CURRENT_VERSION: u8 = 0x01;

    /// Creates a new packet with the given type and sender UUID.
    #[inline]
    pub fn new(label: PacketLabel, source: ClientId) -> Self {
        Self {
            label,
            source,
            sequence: 0,
            payload: vec![],
        }
    }

    /// Obtains the type.
    #[inline]
    pub fn label(&self) -> PacketLabel {
        self.label
    }

    /// Source of the packet.
    #[inline]
    pub fn source(&self) -> ClientId {
        self.source
    }

    /// Sets the source of the packet.
    #[inline]
    pub fn set_source(&mut self, source: ClientId) {
        self.source = source;
    }

    /// Obtains the sequencing number for packet ordering.
    #[allow(dead_code)]
    #[inline]
    pub fn sequence(&self) -> u16 {
        self.sequence
    }

    /// Sets the sequence number of the packet.
    #[inline]
    pub fn set_sequence(&mut self, sequence: u16) {
        self.sequence = sequence;
    }

    /// Obtains the payload of the packet.
    #[inline]
    pub fn payload<T: NetDecoder>(&self) -> Result<T> {
        T::decode(&self.payload)
            .map(|(payload, _)| payload)
            .map_err(|_| NetError::NetCode("Failed to decode payload".to_string()))
    }

    /// Sets the payload of the packet.
    #[inline]
    pub fn set_payload(&mut self, payload: impl NetEncoder) {
        self.payload = payload.encode();
    }
}

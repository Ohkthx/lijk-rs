use super::ClientId;
use super::error::{NetError, Result};
use super::netcode_derive::{NetDecode, NetEncode};
use super::traits::{NetDecoder, NetEncoder};

/// Packet labels for connections that can be sent.
#[derive(PartialEq, Copy, Clone, Debug, NetEncode)]
pub enum PacketLabel {
    Error = 0x00, // Error packet, used to specify an error.
    Acknowledge,  // Acknowledge an action.
    Connect,      // Connect to a server or client.
    Disconnect,   // Disconnect from a server or client.
    Ping,         // Ping packet, used to check if the connection is alive.
    Message,      // Message packet, used to send a message to a server or client.
    Unknown,      // Unknown packet.
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
            _ => Ok((PacketLabel::Unknown, 1)),
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
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Sets the payload of the packet.
    #[inline]
    pub fn set_payload(&mut self, payload: impl NetEncoder) {
        self.payload = payload.encode();
    }
}

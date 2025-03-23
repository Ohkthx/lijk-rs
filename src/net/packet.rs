use crate::{flee, net::ErrorPacket};

use super::{NetError, Result};

// Used to modify the size of the packet header.
type VersionType = u8;
type LabelType = u8;
pub(crate) type SenderType = u16;
pub(crate) type SequenceType = u16;
type Range = std::ops::Range<usize>;

/// Sizes of the packet header fields.
const VERSION_SIZE: usize = size_of::<VersionType>();
const LABEL_SIZE: usize = size_of::<LabelType>();
const SENDER_SIZE: usize = size_of::<SenderType>();
const SEQUENCE_SIZE: usize = size_of::<SequenceType>();

/// Combined size of all header fields.
const HEADER_SIZE: usize = VERSION_SIZE + LABEL_SIZE + SENDER_SIZE + SEQUENCE_SIZE;

// Single-byte fields get an offset constant:
const VERSION_OFFSET: usize = 0;
const LABEL_OFFSET: usize = VERSION_OFFSET + VERSION_SIZE;

/// Ranges for the packet header fields.
const SENDER_RANGE: Range = LABEL_OFFSET + LABEL_SIZE..LABEL_OFFSET + LABEL_SIZE + SENDER_SIZE;
const SEQUENCE_RANGE: Range = SENDER_RANGE.end..SENDER_RANGE.end + SEQUENCE_SIZE;
const PAYLOAD_RANGE: std::ops::RangeFrom<usize> = SEQUENCE_RANGE.end..;

/// Packet labels for connections that can be sent.
#[derive(PartialEq, Copy, Clone, Debug)]
pub enum PacketLabel {
    Error = 0x00, // Error packet, used to specify an error.
    Acknowledge,  // Acknowledge an action.
    Connect,      // Connect to a server or client.
    Disconnect,   // Disconnect from a server or client.
    Heartbeat,    // Heartbeat packet, used to check if the connection is alive.
    Message,      // Message packet, used to send a message to a server or client.
    Unknown,      // Unknown packet.
}

impl From<PacketLabel> for LabelType {
    fn from(label: PacketLabel) -> Self {
        label as LabelType
    }
}

impl From<LabelType> for PacketLabel {
    fn from(value: LabelType) -> PacketLabel {
        match value {
            0x00 => PacketLabel::Error,
            0x01 => PacketLabel::Acknowledge,
            0x02 => PacketLabel::Connect,
            0x03 => PacketLabel::Disconnect,
            0x04 => PacketLabel::Heartbeat,
            0x05 => PacketLabel::Message,
            _ => PacketLabel::Unknown,
        }
    }
}

/// A packet that be sent over a connection.
#[derive(Debug, Clone)]
pub struct Packet {
    version: VersionType,   // Current version of the packet.
    label: PacketLabel,     // Label of the packet.
    sender: SenderType,     // ID of the sender.
    sequence: SequenceType, // Sequence number for ordering packets.
    payload: Vec<u8>,       // Extra payload / data to be sent.
}

impl Packet {
    /// Current version of Packets.
    pub(crate) const CURRENT_VERSION: VersionType = 0x01;

    /// Creates a new packet with the given type and sender UUID.
    #[inline]
    pub fn new(packet_type: PacketLabel, sender: SenderType) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            label: packet_type,
            sender,
            sequence: 0,
            payload: Vec::new(),
        }
    }

    /// Obtains the version of the packet.
    #[inline]
    pub fn version(&self) -> VersionType {
        self.version
    }

    /// Obtains the type.
    #[inline]
    pub fn label(&self) -> PacketLabel {
        self.label
    }

    /// Obtains the Sender's ID.
    #[inline]
    pub fn sender(&self) -> SenderType {
        self.sender
    }

    /// Sets the Sender's ID for the packet.
    #[inline]
    pub fn set_sender(&mut self, id: SenderType) {
        self.sender = id;
    }

    /// Obtains the sequencing number for packet ordering.
    #[inline]
    pub fn sequence(&self) -> SequenceType {
        self.sequence
    }

    /// Sets the sequence number of the packet.
    #[inline]
    pub fn set_sequence(&mut self, sequence: SequenceType) {
        self.sequence = sequence;
    }

    /// Obtains the payload of the packet.
    #[inline]
    pub fn payload(&self) -> &[u8] {
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
        let mut buffer = vec![0; HEADER_SIZE + packet.payload.len()];
        buffer[VERSION_OFFSET] = packet.version;
        buffer[LABEL_OFFSET] = LabelType::from(packet.label);
        buffer[SENDER_RANGE].copy_from_slice(&packet.sender.to_be_bytes());
        buffer[SEQUENCE_RANGE].copy_from_slice(&packet.sequence.to_be_bytes());
        buffer[PAYLOAD_RANGE].copy_from_slice(&packet.payload);
        buffer
    }
}

impl TryFrom<&[u8]> for Packet {
    type Error = NetError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            flee!(NetError::InvalidPacket(
                ErrorPacket::InvalidPacketSize,
                Some(HEADER_SIZE),
                bytes.len()
            ));
        }

        // Parse the header.
        let version = bytes[VERSION_OFFSET];
        if version != Self::CURRENT_VERSION {
            flee!(NetError::InvalidPacket(
                ErrorPacket::InvalidPacketVersion,
                Some(usize::from(Self::CURRENT_VERSION)),
                usize::from(version)
            ));
        }

        let label = PacketLabel::from(bytes[LABEL_OFFSET]);
        if label == PacketLabel::Unknown {
            flee!(NetError::InvalidPacket(
                ErrorPacket::InvalidPacketLabel,
                None,
                usize::from(bytes[LABEL_OFFSET])
            ));
        }

        let sender = SenderType::from_be_bytes(bytes[SENDER_RANGE].try_into().unwrap());
        let sequence = SequenceType::from_be_bytes(bytes[SEQUENCE_RANGE].try_into().unwrap());

        // Remainder is payload.
        let payload = bytes[HEADER_SIZE..].to_vec();

        Ok(Self {
            version,
            label,
            sender,
            sequence,
            payload,
        })
    }
}

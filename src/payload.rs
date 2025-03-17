use std::time::Duration;

use uuid::Uuid;

use crate::net::{Packet, PacketType};

/// Exmaple of a payload from a packet.
pub enum Payload {
    None,                // Represents an empty payload.
    String(String),      // Represents a string payload.
    Uuid(Uuid),          // Represents a UUID payload.
    Timestamp(Duration), // Represents a timestamp payload.
}

impl Payload {
    /// Converts the payload to a byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        Vec::from(self)
    }
}

impl From<&Packet> for Payload {
    fn from(value: &Packet) -> Self {
        let raw = value.get_payload();
        if raw.is_empty() {
            return Self::None;
        }

        match value.get_type() {
            PacketType::Error | PacketType::Message => {
                Self::String(String::from_utf8_lossy(raw).to_string())
            }
            PacketType::Connect => {
                if raw.len() == 16 {
                    let uuid = Uuid::from_slice(raw).unwrap();
                    Self::Uuid(uuid)
                } else {
                    Self::None
                }
            }
            PacketType::Heartbeat => {
                if raw.len() == 12 {
                    let ts = Timestamp::from(raw);
                    Self::Timestamp(Duration::from(&ts))
                } else {
                    Self::None
                }
            }
            _ => Self::None,
        }
    }
}

impl From<Packet> for Payload {
    fn from(value: Packet) -> Self {
        Self::from(&value)
    }
}

impl From<&Payload> for Vec<u8> {
    fn from(value: &Payload) -> Self {
        match value {
            Payload::None => vec![],
            Payload::String(s) => s.as_bytes().to_vec(),
            Payload::Uuid(uuid) => Vec::from(uuid.as_bytes()),
            Payload::Timestamp(ts) => {
                let ts = Timestamp(ts.as_secs(), ts.subsec_nanos());
                Vec::from(ts)
            }
        }
    }
}

/// Represents a timestamp in seconds and nanoseconds. Essentially a Duration.
pub struct Timestamp(pub u64, pub u32);

impl From<&[u8]> for Timestamp {
    fn from(value: &[u8]) -> Self {
        let seconds = u64::from_be_bytes(value[0..8].try_into().unwrap());
        let nanos = u32::from_be_bytes(value[8..12].try_into().unwrap());
        Timestamp(seconds, nanos)
    }
}

impl From<&Timestamp> for Duration {
    fn from(value: &Timestamp) -> Self {
        Duration::from_secs(value.0) + Duration::from_nanos(u64::from(value.1))
    }
}

impl From<Timestamp> for Vec<u8> {
    fn from(value: Timestamp) -> Self {
        let mut bytes = vec![0; 12];
        bytes[0..8].copy_from_slice(&value.0.to_be_bytes());
        bytes[8..12].copy_from_slice(&value.1.to_be_bytes());
        bytes
    }
}

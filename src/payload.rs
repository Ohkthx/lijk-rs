use std::time::Duration;

use crate::net::{Packet, PacketType};

/// Exmaple of a payload from a packet.
pub enum Payload {
    None,                      // Represents an empty payload.
    Error(u8, Option<String>), // Represents an error payload with a code and message.
    String(String),            // Represents a string payload.
    U32(u32),                  // Represents a 32-bit unsigned integer payload.
    Timestamp(bool, Duration), // Represents a timestamp payload.
}

impl From<&Packet> for Payload {
    fn from(value: &Packet) -> Self {
        let raw = value.get_payload();
        if raw.is_empty() {
            return Self::None;
        }

        match value.get_type() {
            PacketType::Error => {
                if raw.is_empty() {
                    Self::None
                } else {
                    let code = raw[0];
                    let message = if raw.len() > 1 {
                        Some(String::from_utf8_lossy(&raw[1..]).to_string())
                    } else {
                        None
                    };
                    Self::Error(code, message)
                }
            }
            PacketType::Message => Self::String(String::from_utf8_lossy(raw).to_string()),
            PacketType::Connect => {
                if raw.len() == size_of::<u32>() {
                    let id = u32::from_be_bytes(raw.try_into().unwrap());
                    Self::U32(id)
                } else {
                    Self::None
                }
            }
            PacketType::Heartbeat => {
                if raw.len() == size_of::<bool>() + size_of::<u64>() + size_of::<u32>() {
                    let respond = raw[0] != 0;
                    let ts = Timestamp::from(&raw[1..13]);
                    Self::Timestamp(respond, Duration::from(&ts))
                } else {
                    Self::None
                }
            }
            _ => Self::None,
        }
    }
}

impl From<&Payload> for Vec<u8> {
    fn from(value: &Payload) -> Self {
        match value {
            Payload::None => vec![],
            Payload::Error(code, message) => {
                let mut bytes = vec![*code];
                if let Some(msg) = message {
                    bytes.extend_from_slice(msg.as_bytes());
                }
                bytes
            }
            Payload::String(s) => s.as_bytes().to_vec(),
            Payload::U32(id) => Vec::from(&id.to_be_bytes()),
            Payload::Timestamp(respond, ts) => {
                let mut bytes = vec![u8::from(*respond)];
                bytes.extend_from_slice(&Timestamp(ts.as_secs(), ts.subsec_nanos()).as_bytes());
                bytes
            }
        }
    }
}

impl From<Payload> for Vec<u8> {
    fn from(value: Payload) -> Self {
        Vec::from(&value)
    }
}

/// Represents a timestamp in seconds and nanoseconds. Essentially a Duration.
struct Timestamp(pub u64, pub u32);

impl Timestamp {
    /// Converts the timestamp to a byte vector.
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0; 12];
        bytes[0..8].copy_from_slice(&self.0.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.1.to_be_bytes());
        bytes
    }
}

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

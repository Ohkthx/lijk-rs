use crate::net::traits::{NetDecoder, NetEncoder};
use crate::vec2f::Vec2f;
use netcode_derive::{NetDecode, NetEncode};

#[repr(u8)]
pub enum PayloadId {
    Connect = 0x06,
    State,
    Position,
    Movement,
    Unknown,
}

impl From<u8> for PayloadId {
    fn from(value: u8) -> Self {
        match value {
            0x06 => PayloadId::Connect,
            0x07 => PayloadId::State,
            0x08 => PayloadId::Position,
            0x09 => PayloadId::Movement,
            _ => PayloadId::Unknown,
        }
    }
}

impl From<PayloadId> for u8 {
    fn from(value: PayloadId) -> Self {
        match value {
            PayloadId::Connect => 0x06,
            PayloadId::State => 0x07,
            PayloadId::Position => 0x08,
            PayloadId::Movement => 0x09,
            PayloadId::Unknown => 0xFF,
        }
    }
}

/// Sent from a server containing the Entity Id and position.
#[derive(NetDecode, NetEncode, Debug, Clone, Copy)]
pub struct Connect(pub u32, pub Vec2f);

/// Current state of the server including the ticks-per-second and current tick Id.
#[derive(NetDecode, NetEncode, Debug, Clone, Copy)]
pub struct ServerState {
    pub tps: u16,
    pub tick_id: u64,
}

/// Represents an Entity ID, position, and velocity.
#[derive(NetDecode, NetEncode, Debug, Clone, Copy)]
pub struct Position(pub u32, pub Vec2f, pub Vec2f);

/// Represents a movement command with a movement delta and speed.
#[derive(NetDecode, NetEncode, Debug, Clone, Copy)]
pub struct Movement(pub Vec2f, pub u8);

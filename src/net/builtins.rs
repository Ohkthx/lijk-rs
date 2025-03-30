use std::time::Duration;

use super::ClientId;
use super::error::ErrorPacket;
use super::netcode_derive::{NetDecode, NetEncode};
use super::traits::{NetDecoder, NetEncoder};

/// Built-in Connection payload.
///
/// # Fields
/// - `u8`: The packet version.
/// - `ClientId`: The ID of the client.
/// - `u64`: Amount of time in milliseconds to send ping.
#[derive(NetEncode, NetDecode, Debug)]
pub struct ConnectionPayload(pub u8, pub ClientId, pub u64);

/// Built-in Ping payload.
///
/// # Fields
/// - `Duration`: The duration of the ping.
/// - `bool`: A boolean value indicating to reply or not.
#[derive(NetEncode, NetDecode, Debug)]
pub struct PingPayload(pub Duration, pub bool);

/// Built-in Error payload.
///
/// # Fields
/// - `ErrorPacket`: The error packet code.
/// - `Option<String>`: An optional string message.
#[derive(NetEncode, NetDecode, Debug)]
pub struct ErrorPayload(pub ErrorPacket, pub String);

/// Built-in Message payload.
///
/// # Fields
/// - `String`: The message string.
#[derive(NetEncode, NetDecode, Debug)]
pub struct MessagePayload(pub String);

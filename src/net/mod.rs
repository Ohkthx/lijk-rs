mod client;
mod error;
mod local;
mod packet;
mod remote;
mod socket;
mod storage;

pub(crate) use local::LocalSocket;
pub(crate) use packet::{SenderType as EntityId, SequenceType as SequenceId};
pub(crate) use remote::RemoteSocket;

pub use client::ClientAddr;
use error::Result;
pub use error::{ErrorPacket, NetError};
pub use packet::{Packet, PacketLabel};
pub use socket::Socket;
use storage::ClientStorage;

/// ID for a server.
pub(crate) const SERVER_ID: EntityId = 0;
/// Invalid ID for a client.
pub(crate) const INVALID_CLIENT_ID: EntityId = EntityId::MAX;

/// Used to specify the destination and packet for a socket action.
pub struct Deliverable {
    pub(crate) to: EntityId,   // ID of the destination user.
    pub(crate) packet: Packet, // Packet to be sent to the destination.
}

impl Deliverable {
    /// Creates a new deliverable with the given destination and packet.
    pub fn new(to: EntityId, packet: Packet) -> Self {
        Self { to, packet }
    }
}

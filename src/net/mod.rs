mod error;
mod local;
mod packet;
mod remote;
mod socket;
mod storage;

pub(crate) use local::LocalSocket;
pub(crate) use remote::RemoteSocket;

use error::Result;
pub use error::{ErrorPacket, NetError};
pub use packet::{Packet, PacketType};
pub use socket::Socket;
use storage::ClientStorage;

/// ID for the server.
pub(crate) const SERVER_ID: u32 = 0;
/// Invalid ID for a client.
pub(crate) const INVALID_CLIENT_ID: u32 = ClientStorage::<()>::INVALID_CLIENT_ID;

/// Used to specify the destination and packet for a socket action.
pub struct Deliverable {
    pub(crate) to: u32,        // ID of the destination user.
    pub(crate) packet: Packet, // Packet to be sent to the destination.
}

impl Deliverable {
    /// Creates a new deliverable with the given destination and packet.
    pub fn new(to: u32, packet: Packet) -> Self {
        Self { to, packet }
    }
}

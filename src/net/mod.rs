mod client;
mod local;
mod opts;
mod packet;
mod remote;
mod socket;
mod task;

pub mod builtins;
pub mod error;
pub mod storage;
pub mod traits;

pub(crate) use local::LocalSocket;
pub(crate) use remote::RemoteSocket;

pub use netcode_derive;

pub use client::{ClientAddr, ClientId};
pub use opts::SocketOptions;
pub use packet::{Packet, PacketLabel};
pub use socket::Socket;

/// Used to specify the destination and packet for a socket action.
pub struct Deliverable {
    pub(crate) to: ClientId,   // ID of the destination user.
    pub(crate) packet: Packet, // Packet to be sent to the destination.
}

impl Deliverable {
    /// Creates a new deliverable with the given destination and packet.
    pub fn new(to: ClientId, packet: Packet) -> Self {
        Self { to, packet }
    }
}

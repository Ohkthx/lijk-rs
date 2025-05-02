mod macros;
mod sset;
mod timestep;

pub use sset::SparseSet;
pub use timestep::Timestep;

use crate::error::AppError;
use crate::net::{Packet, traits::NetDecoder};

/// Decodes a packet into a specific `P` payload type.
pub fn decode<P: NetDecoder>(packet: &Packet) -> Result<P, AppError> {
    packet.payload::<P>().map_err(AppError::Net)
}

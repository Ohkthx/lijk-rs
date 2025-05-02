use std::time::Duration;

use super::error::{NetError, Result};
use super::{ClientAddr, Packet};

/// Trait for handling packets.
pub(crate) trait SocketHandler {
    /// Send a packet to the connection.
    #[allow(dead_code)]
    fn send(&self, dest: &ClientAddr, packet: Packet) -> Result<()>;
    /// Try to receive a packet from the connection.
    #[allow(dead_code)]
    fn try_recv(&mut self) -> Result<Option<(ClientAddr, Packet)>>;
    /// Waits to receive a packet from the connection.
    #[allow(dead_code)]
    fn recv(&mut self) -> Result<Option<(ClientAddr, Packet)>>;
}

/// Custom encoder to send a packet over the network.
pub trait NetEncoder {
    /// Encodes the object into a byte vector.
    fn encode(self) -> Vec<u8>;
}

/// Custom decoder to receive a packet from the network.
pub trait NetDecoder: Sized {
    /// Decodes the object from a byte slice. Returns a tuple of the decoded object and the number of bytes consumed.
    fn decode(data: &[u8]) -> Result<(Self, usize)>;
}

#[macro_export]
macro_rules! impl_netcode {
    ($($t:ty),*) => {
        $(
            impl NetEncoder for $t {
                fn encode(self) -> Vec<u8> {
                    self.to_be_bytes().to_vec()
                }
            }

            impl NetDecoder for $t {
                fn decode(data: &[u8]) -> std::result::Result<(Self, usize), $crate::net::error::NetError> {
                    if data.len() < ::std::mem::size_of::<$t>() {
                        return Err($crate::net::error::NetError::NetCode(format!(
                            "Not enough bytes to decode {} (need {}, got {})",
                            stringify!($t),
                            ::std::mem::size_of::<$t>(),
                            data.len()
                        )));
                    }

                    let mut bytes = [0u8; ::std::mem::size_of::<$t>()];
                    bytes.copy_from_slice(&data[..::std::mem::size_of::<$t>()]);
                    Ok((<$t>::from_be_bytes(bytes), ::std::mem::size_of::<$t>()))
                }
            }
        )*
    };
}

impl_netcode!(
    u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64, usize, isize
);

impl NetEncoder for bool {
    fn encode(self) -> Vec<u8> {
        vec![u8::from(self)]
    }
}

impl NetDecoder for bool {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(crate::net::error::NetError::NetCode(format!(
                "Not enough bytes to decode bool (need 1, got {})",
                data.len()
            )));
        }
        Ok((data[0] != 0, 1))
    }
}

impl NetEncoder for Vec<u8> {
    fn encode(self) -> Vec<u8> {
        self
    }
}

impl NetDecoder for Vec<u8> {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        Ok((data.to_vec(), data.len()))
    }
}

impl NetEncoder for &[u8] {
    fn encode(self) -> Vec<u8> {
        self.to_vec()
    }
}

impl NetEncoder for String {
    fn encode(self) -> Vec<u8> {
        self.into_bytes()
    }
}

impl NetDecoder for String {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.is_empty() {
            return Err(NetError::NetCode(format!(
                "Not enough bytes to decode String (need 1, got {})",
                data.len()
            )));
        }

        let string = String::from_utf8(data.to_vec())
            .map_err(|_| NetError::NetCode("Failed to decode String from bytes".to_string()))?;
        Ok((string, data.len()))
    }
}

impl<T: NetEncoder> NetEncoder for Option<T> {
    fn encode(self) -> Vec<u8> {
        match self {
            Some(inner) => {
                let mut out = vec![1]; // Mark presence.
                out.extend(inner.encode()); // Encode the inner T.
                out
            }
            None => {
                vec![0] // Mark absence.
            }
        }
    }
}

impl<T: NetDecoder> NetDecoder for Option<T> {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        // Need at least 1 byte to see if it's Some or None
        if data.is_empty() {
            return Err(NetError::NetCode(
                "Not enough bytes to decode Option".to_string(),
            ));
        }

        if data[0] == 0 {
            Ok((None, 1)) // 0 means None, read 1 byte.
        } else {
            let (value, size) = T::decode(&data[1..])?;
            Ok((Some(value), size + 1)) // +1 for the tag byte.
        }
    }
}

impl NetEncoder for Duration {
    fn encode(self) -> Vec<u8> {
        let mut out = vec![0; 12];
        out[0..8].copy_from_slice(&self.as_secs().to_be_bytes());
        out[8..12].copy_from_slice(&self.subsec_nanos().to_be_bytes());
        out
    }
}

impl NetDecoder for Duration {
    fn decode(data: &[u8]) -> Result<(Self, usize)> {
        if data.len() < 12 {
            return Err(NetError::NetCode(format!(
                "Not enough bytes to decode Duration (need 12, got {})",
                data.len()
            )));
        }

        let secs = u64::from_be_bytes(data[0..8].try_into().unwrap());
        let nanos = u32::from_be_bytes(data[8..12].try_into().unwrap());
        Ok((Duration::new(secs, nanos), 12))
    }
}

impl NetEncoder for () {
    fn encode(self) -> Vec<u8> {
        vec![]
    }
}

impl NetDecoder for () {
    fn decode(_data: &[u8]) -> Result<(Self, usize)> {
        Ok(((), 0))
    }
}

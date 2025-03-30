use std::net::{IpAddr, SocketAddr};

use super::netcode_derive::{NetDecode, NetEncode};
use super::traits::{NetDecoder, NetEncoder};

/// Represents a client ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, NetEncode, NetDecode, Hash, PartialOrd, Ord)]
pub struct ClientId(pub(crate) u16);

impl ClientId {
    /// Invalid Client ID.
    pub const INVALID: Self = ClientId(u16::MAX);
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId({})", self.0)
    }
}

impl From<ClientId> for usize {
    fn from(client_id: ClientId) -> Self {
        usize::from(client_id.0)
    }
}

impl TryFrom<usize> for ClientId {
    type Error = usize;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        if value > u16::MAX as usize {
            Err(value)
        } else {
            Ok(ClientId(u16::try_from(value).unwrap()))
        }
    }
}

/// Represents a client address, either local or remote.
#[derive(Debug, Clone, Copy)]
pub enum ClientAddr {
    Local(ClientId), // Local client ID.
    Ip(IpAddr, u16), // Remote client IP address and port.
}

impl PartialEq for ClientAddr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ClientAddr::Local(id1), ClientAddr::Local(id2)) => id1 == id2,
            #[cfg(feature = "shared_ip")]
            (ClientAddr::Ip(ip1, port1), ClientAddr::Ip(ip2, port2)) => {
                ip1 == ip2 && port1 == port2
            }
            #[cfg(not(feature = "shared_ip"))]
            (ClientAddr::Ip(ip1, _), ClientAddr::Ip(ip2, _)) => ip1 == ip2,
            _ => false,
        }
    }
}

impl Eq for ClientAddr {}

impl std::hash::Hash for ClientAddr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ClientAddr::Local(client_id) => client_id.hash(state),
            #[cfg(feature = "shared_ip")]
            ClientAddr::Ip(ip, port) => {
                ip.hash(state);
                port.hash(state);
            }
            #[cfg(not(feature = "shared_ip"))]
            ClientAddr::Ip(ip, _) => {
                ip.hash(state);
            }
        }
    }
}

impl std::fmt::Display for ClientAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientAddr::Local(id) => write!(f, "Local({id})"),
            #[cfg(feature = "shared_ip")]
            ClientAddr::Ip(ip, port) => write!(f, "Ip({ip}, {port})"),
            #[cfg(not(feature = "shared_ip"))]
            ClientAddr::Ip(ip, _) => write!(f, "Ip({ip})"),
        }
    }
}

impl From<SocketAddr> for ClientAddr {
    fn from(addr: SocketAddr) -> Self {
        ClientAddr::Ip(addr.ip(), addr.port())
    }
}

impl From<ClientId> for ClientAddr {
    fn from(client_id: ClientId) -> Self {
        ClientAddr::Local(client_id)
    }
}

use std::net::{IpAddr, SocketAddr};

use super::EntityId;

/// Represents a client address, either local or remote.
#[derive(Debug, Clone, Copy)]
pub enum ClientAddr {
    Local(EntityId), // Local client ID.
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
            ClientAddr::Local(id) => id.hash(state),
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
            ClientAddr::Ip(ip, port) => write!(f, "Ip({ip}, {port})"),
        }
    }
}

impl From<SocketAddr> for ClientAddr {
    fn from(addr: SocketAddr) -> Self {
        ClientAddr::Ip(addr.ip(), addr.port())
    }
}

impl From<EntityId> for ClientAddr {
    fn from(id: EntityId) -> Self {
        ClientAddr::Local(id)
    }
}

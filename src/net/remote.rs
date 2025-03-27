use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use crate::flee;

use super::socket::SocketHandler;
use super::{ClientAddr, NetError, Packet, Result};

/// Remote connection that uses UDP to communicate with a remote server or client.
pub(crate) struct RemoteSocket {
    socket: UdpSocket, // Raw socket.

    local_addr: String, // Local address for the socket.

    buffer: [u8; 1024], // Buffer for receiving data.
    nonblocking: bool,  // Used to track if the socket is in non-blocking mode.
}

impl RemoteSocket {
    /// Default addresses for the server.
    pub(crate) const DEFAULT_SERVER_ADDR: &'static str = "127.0.0.1:31013";
    /// Default address for the client to bind to. This is used when the client does not have a specific address.
    pub(crate) const DEFAULT_CLIENT_ADDR: &'static str = "0.0.0.0:0";

    /// Creates a new remote connection with the given address.
    pub(crate) fn new(is_server: bool) -> Result<Self> {
        let addr = if is_server {
            Self::DEFAULT_SERVER_ADDR
        } else {
            Self::DEFAULT_CLIENT_ADDR
        };

        // Bind the socket to the address.
        let socket = match UdpSocket::bind(addr) {
            Ok(socket) => socket,
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        };

        let mut connection = Self {
            socket,
            local_addr: addr.to_string(),

            buffer: [0; 1024],
            nonblocking: false,
        };

        // Set the socket to non-blocking mode.
        connection.toggle_nonblocking()?;

        Ok(connection)
    }

    /// Obtains the address of the socket.
    #[inline]
    pub(crate) fn address(&self) -> &str {
        &self.local_addr
    }

    /// Toggles between blocking and non-blocking modes.
    fn toggle_nonblocking(&mut self) -> Result<()> {
        self.nonblocking = !self.nonblocking;
        if let Err(why) = self.socket.set_nonblocking(self.nonblocking) {
            self.nonblocking = !self.nonblocking; // Reset if an error occurs.
            flee!(NetError::SocketError(why.to_string()));
        }

        Ok(())
    }

    /// Wraps the `send_to` method to send a packet to a specific address.
    fn send_to<T: ToSocketAddrs>(&self, packet: &Packet, addr: &T) -> Result<()> {
        if let Err(why) = self.socket.send_to(&Vec::from(packet), addr) {
            flee!(NetError::SocketError(format!(
                "Unable to send packet: {why}",
            )));
        }

        Ok(())
    }
}

impl SocketHandler for RemoteSocket {
    #[inline]
    fn send(&mut self, dest: &ClientAddr, packet: Packet) -> Result<()> {
        if let ClientAddr::Ip(ip, port) = dest {
            self.send_to(&packet, &SocketAddr::new(*ip, *port))
        } else {
            flee!(NetError::NotConnected(*dest, true));
        }
    }

    #[inline]
    fn try_recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        if !self.nonblocking {
            self.toggle_nonblocking()?;
        }

        match self.socket.recv_from(&mut self.buffer) {
            Ok((size, sender)) => {
                // Parse the packet and client.
                let packet = match Packet::try_from(&self.buffer[..size]) {
                    Ok(packet) => packet,
                    Err(NetError::InvalidPacketParse(err, expected, got)) => {
                        // Wraps the error to provide more context.
                        flee!(NetError::InvalidPacket(
                            ClientAddr::Ip(sender.ip(), sender.port()),
                            err,
                            expected,
                            got
                        ))
                    }
                    Err(why) => flee!(why),
                };
                Ok(Some((ClientAddr::Ip(sender.ip(), sender.port()), packet)))
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Ok(None) // No data available, return None.
            }
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        }
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        if self.nonblocking {
            self.toggle_nonblocking()?;
        }

        match self.socket.recv_from(&mut self.buffer) {
            Ok((size, sender)) => {
                // Parse the packet and client.
                let packet = match Packet::try_from(&self.buffer[..size]) {
                    Ok(packet) => packet,
                    Err(NetError::InvalidPacketParse(err, expected, got)) => {
                        // Wraps the error to provide more context.
                        flee!(NetError::InvalidPacket(
                            ClientAddr::Ip(sender.ip(), sender.port()),
                            err,
                            expected,
                            got
                        ))
                    }
                    Err(why) => flee!(why),
                };
                Ok(Some((ClientAddr::Ip(sender.ip(), sender.port()), packet)))
            }
            Err(why) => flee!(NetError::SocketError(why.to_string())),
        }
    }
}

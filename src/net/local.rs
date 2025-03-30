use std::sync::mpsc;

use crate::flee;

use super::error::{NetError, Result};
use super::traits::SocketHandler;
use super::{ClientAddr, Packet};

/// Local connection that uses MPSC to communicate locally.
pub(crate) struct LocalSocket {
    tx: Option<mpsc::Sender<Packet>>,   // Sender for the connection.
    rx: Option<mpsc::Receiver<Packet>>, // Receiver for the connection.
}

impl LocalSocket {
    /// Creates a new local connection.
    pub(crate) fn new() -> Self {
        Self { tx: None, rx: None }
    }

    /// Creates the receiver for the connection.
    pub(crate) fn create_rx(&mut self) -> Result<mpsc::Receiver<Packet>> {
        if self.tx.is_some() {
            flee!(NetError::SocketError(
                "Cannot create receiver when sender already exists".to_string()
            ));
        }

        let (tx, rx) = mpsc::channel::<Packet>();
        self.tx = Some(tx);
        Ok(rx)
    }

    /// Sets the receiver for the connection.
    pub(crate) fn set_rx(&mut self, rx: mpsc::Receiver<Packet>) -> Result<()> {
        if self.rx.is_some() {
            flee!(NetError::SocketError(
                "Cannot set receiver when one already exists".to_string()
            ));
        }

        self.rx = Some(rx);
        Ok(())
    }

    /// Obtains the address of the socket.
    #[inline]
    pub(crate) fn address() -> &'static str {
        "localhost"
    }
}

impl SocketHandler for LocalSocket {
    #[inline]
    fn send(&self, _dest: &ClientAddr, packet: Packet) -> Result<()> {
        if let Some(sender) = &self.tx {
            sender
                .send(packet)
                .map_err(|_| NetError::SocketError("Failed to send packet".to_string()))?;
            Ok(())
        } else {
            flee!(NetError::SocketError(
                "Cannot send to a socket that has no sender".to_string()
            ));
        }
    }

    #[inline]
    fn try_recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        if let Some(rx) = &self.rx {
            match rx.try_recv() {
                Ok(packet) => Ok(Some((ClientAddr::Local(packet.source()), packet))),
                Err(mpsc::TryRecvError::Empty) => Ok(None),
                Err(mpsc::TryRecvError::Disconnected) => flee!(NetError::Disconnected),
            }
        } else {
            flee!(NetError::SocketError(
                "Cannot receive from a socket that has no receiver".to_string()
            ));
        }
    }

    #[inline]
    fn recv(&mut self) -> Result<Option<(ClientAddr, Packet)>> {
        if let Some(rx) = &self.rx {
            match rx.recv() {
                Ok(packet) => Ok(Some((ClientAddr::Local(packet.source()), packet))),
                Err(_) => flee!(NetError::Disconnected),
            }
        } else {
            flee!(NetError::SocketError(
                "Cannot receive from a socket that has no receiver".to_string()
            ));
        }
    }
}

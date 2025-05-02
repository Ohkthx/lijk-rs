use std::time::Duration;

use crate::error::AppError;
use crate::net::builtins::{ConnectionPayload, ErrorPayload, MessagePayload};
use crate::net::error::NetError;
use crate::net::traits::NetEncoder;
use crate::net::{ClientId, Deliverable, Packet, PacketLabel, Socket};
use crate::utils::decode;
use crate::{Result, debugln, flee};

/// Basic client implementation that connects to a server.
pub struct ClientSocket {
    socket: Socket,   // The socket used for communication.
    server: ClientId, // The ID of the server to connect to.
}

impl ClientSocket {
    /// Maximum number of connection retries before disconnecting.
    const MAX_CONNECTION_RETRY: u8 = 30;

    /// Creates a new client with the given connection.
    pub fn new(socket: Socket) -> Self {
        Self {
            socket,
            server: ClientId::INVALID,
        }
    }

    /// Obtains the ID of the client.
    #[inline]
    pub fn id(&self) -> ClientId {
        self.socket.id()
    }

    /// Sends a packet to the server.
    pub fn send(
        &mut self,
        packet_type: PacketLabel,
        payload: Option<impl NetEncoder>,
    ) -> Result<()> {
        let mut packet = Packet::new(packet_type, self.id());
        if let Some(data) = payload {
            packet.set_payload(data);
        }

        match self.socket.send(Deliverable::new(self.server, packet)) {
            Ok(()) => Ok(()),
            Err(NetError::SocketError(why)) => Err(AppError::Net(NetError::SocketError(why))),
            Err(why) => {
                debugln!("CLIENT: Failed to send packet to server: {}", why);
                Ok(())
            }
        }
    }

    /// Waits for a connection to be established with the server.
    pub fn wait_for_connection(&mut self) -> Result<()> {
        let mut retry_count = 0;
        while retry_count < Self::MAX_CONNECTION_RETRY && self.server == ClientId::INVALID {
            // Send a connect packet to the server.
            let payload = ConnectionPayload(Packet::CURRENT_VERSION, self.id(), 5000);
            self.send(PacketLabel::Connect, Some(payload))?;
            std::thread::sleep(Duration::from_millis(500));

            self.packet_processor(&mut vec![])?;
            retry_count += 1;
        }

        // Check if a connection was never established.
        if retry_count >= Self::MAX_CONNECTION_RETRY {
            flee!(AppError::Net(NetError::SocketError(format!(
                "Failed to establish connection to server after {} attempts",
                Self::MAX_CONNECTION_RETRY
            ))));
        } else if self.server == ClientId::INVALID {
            flee!(AppError::Net(NetError::SocketError(
                "Failed to establish connection to server, no response received.".to_string()
            )));
        }

        Ok(())
    }

    /// Runs a single step of the client, processing packets and handling timeouts.
    pub fn run_step(&mut self) -> Result<Vec<Packet>> {
        let mut out = vec![];
        while self.packet_processor(&mut out)?.is_some() {}
        self.socket.run_tasks(false).map_err(AppError::Net)?;

        Ok(out)
    }

    /// Processes incoming packets and handles different packet types.
    fn packet_processor(
        &mut self,
        out: &mut Vec<Packet>,
    ) -> std::result::Result<Option<()>, AppError> {
        let packet = match self.socket.try_recv() {
            Ok(Some(packet)) => packet,
            Ok(None) => return Ok(None),
            Err(NetError::SocketError(why)) => Err(AppError::Net(NetError::SocketError(why)))?,
            Err(why) => {
                debugln!("CLIENT: Obtaining packet error: {}", why);
                return Ok(None);
            }
        };

        match packet.label() {
            PacketLabel::Error => {
                let payload = decode::<ErrorPayload>(&packet)?;
                debugln!(
                    "CLIENT: [{}] Received error: {:?}",
                    packet.source(),
                    payload
                );
            }

            PacketLabel::Acknowledge => {
                debugln!("CLIENT: [{}] Received acknowledge.", self.id());
            }

            PacketLabel::Connect => {
                let payload = decode::<ConnectionPayload>(&packet)?;
                self.server = packet.source();
                debugln!(
                    "CLIENT: [{}] Connected, Server: {}. Payload: {:?}",
                    self.id(),
                    self.server,
                    payload
                );
            }

            PacketLabel::Disconnect => {
                debugln!("CLIENT: [{}] Server sent disconnect command.", self.id());

                if !self.socket.is_remote() {
                    // Notify server for safe shutdown on local sockets.
                    self.send(PacketLabel::Disconnect, None::<()>)?;
                }

                flee!(AppError::Net(NetError::Disconnected));
            }

            PacketLabel::Ping => {
                // let payload = packet.payload::<PingPayload>()?;
                // debugln!("CLIENT: [{}] Received ping {:?}", packet.source(), payload);
            }

            PacketLabel::Message => {
                let payload = decode::<MessagePayload>(&packet)?;
                debugln!("CLIENT: [{}] Received message: {:?}", self.id(), payload);
            }

            PacketLabel::Extension(_value) => {}
        }

        out.push(packet);
        Ok(Some(()))
    }
}

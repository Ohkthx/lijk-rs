use crate::error::{AppError, Result};
use crate::net::builtins::{ErrorPayload, MessagePayload, PingPayload};
use crate::net::error::NetError;
use crate::net::traits::{NetDecoder, NetEncoder};
use crate::net::{ClientId, Deliverable, Packet, PacketLabel, Socket};
use crate::{debugln, flee};

/// Basic server implementation that can handle multiple clients.
pub struct Server {
    socket: Socket, // The socket used for communication.
}

impl Server {
    /// Creates a new server with the given connection.
    pub fn new(socket: Socket) -> Self {
        Self { socket }
    }

    /// Obtains the ID of the server.
    #[allow(dead_code)]
    #[inline]
    fn id(&self) -> ClientId {
        self.socket.id()
    }

    /// Decpodes a packet payload into the specified type.
    pub fn decode<T: NetDecoder>(packet: &Packet) -> Result<T> {
        T::decode(packet.payload())
            .map(|(payload, _)| payload)
            .map_err(AppError::NetError)
    }

    /// Sends a packet to the client.
    #[allow(dead_code)]
    fn send(
        &mut self,
        packet_type: PacketLabel,
        dest: ClientId,
        payload: Option<impl NetEncoder>,
    ) -> Result<()> {
        let mut packet = Packet::new(packet_type, self.id());
        if let Some(data) = payload {
            packet.set_payload(data);
        }

        match self.socket.send(Deliverable::new(dest, packet)) {
            Ok(()) => Ok(()),
            Err(NetError::SocketError(why)) => Err(AppError::NetError(NetError::SocketError(why))),
            Err(why) => {
                debugln!(
                    "SERVER: Failed to send packet to client [{}]: {}",
                    dest,
                    why
                );
                Ok(())
            }
        }
    }

    /// Disconnects a client from the server and removes it from the list.
    fn disconnect_client(&mut self, id: ClientId, notify: bool) -> Result<()> {
        // Remove the client from the list.
        match self.socket.disconnect_client(id, notify) {
            Ok(()) => Ok(()),
            Err(NetError::SocketError(why)) => Err(AppError::NetError(NetError::SocketError(why))),
            Err(why) => {
                debugln!("SERVER: Error while disconnecting client [{}]: {}", id, why);
                Ok(())
            }
        }
    }

    /// Runs a single step of the server, processing incoming packets.
    #[inline]
    pub fn run_step(&mut self) -> Result<()> {
        // Process all incoming packets until none remain.
        while self.packet_processor()?.is_some() {}
        self.socket.run_tasks(false).map_err(AppError::NetError)?;

        Ok(())
    }

    /// Processes incoming packets and handles their types.
    fn packet_processor(&mut self) -> Result<Option<Packet>> {
        // Receive the packet from the socket.
        let packet = match self.socket.try_recv() {
            Ok(Some(packet)) => packet,
            Ok(None) | Err(NetError::InvalidPacket(..) | NetError::NothingToDo) => return Ok(None),
            Err(NetError::SocketError(why)) => Err(AppError::NetError(NetError::SocketError(why)))?,
            Err(why) => {
                debugln!("SERVER: Failed to receive packet: {}", why);
                return Ok(None);
            }
        };

        match packet.label() {
            PacketLabel::Error => {
                let payload = Self::decode::<ErrorPayload>(&packet)?;
                debugln!(
                    "SERVER: [{}] Received error: {:?}",
                    packet.source(),
                    payload
                );
            }

            PacketLabel::Acknowledge => {
                debugln!("SERVER: [{}] Received acknowledge.", packet.source());
            }

            PacketLabel::Connect => {
                debugln!("SERVER: [{}] Client connected.", packet.source());
            }

            PacketLabel::Disconnect => {
                debugln!("SERVER: Client [{}] is disconnecting.", packet.source(),);
                self.disconnect_client(packet.source(), false)?;
                if !self.socket.is_remote() {
                    // Local sockets shut the server down on disconnect.
                    flee!(AppError::NetError(NetError::Disconnected));
                }
            }

            PacketLabel::Ping => {
                let payload = Self::decode::<PingPayload>(&packet)?;
                debugln!("SERVER: [{}] Received ping {:?}", packet.source(), payload);
            }

            PacketLabel::Message => {
                let payload = Self::decode::<MessagePayload>(&packet)?;
                debugln!(
                    "SERVER: [{}] Received message: {:?}",
                    packet.source(),
                    payload
                );
            }

            PacketLabel::Unknown => {
                debugln!(
                    "SERVER: [{}] Received unknown packet label: {:?}.",
                    packet.source(),
                    packet.label()
                );
            }
        }

        Ok(Some(packet))
    }
}

use crate::error::{AppError, Result};
use crate::net::error::NetError;
use crate::net::{ClientId, Deliverable, Packet, PacketLabel, Socket};
use crate::{debugln, flee};

/// Basic server implementation that can handle multiple clients.
pub struct ServerSocket {
    socket: Socket, // The socket used for communication.
}

impl ServerSocket {
    /// Creates a new server with the given connection.
    pub fn new(socket: Socket) -> Self {
        Self { socket }
    }

    /// Obtains the ID of the server.
    #[allow(dead_code)]
    #[inline]
    pub fn id(&self) -> ClientId {
        self.socket.id()
    }

    /// Sends a packet to the client.
    #[allow(dead_code)]
    pub fn send(&mut self, dest: ClientId, packet: Packet) -> Result<()> {
        match self.socket.send(Deliverable::new(dest, packet)) {
            Ok(()) => Ok(()),
            Err(NetError::SocketError(why)) => Err(AppError::Net(NetError::SocketError(why))),
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
            Err(NetError::SocketError(why)) => Err(AppError::Net(NetError::SocketError(why))),
            Err(why) => {
                debugln!("SERVER: Error while disconnecting client [{}]: {}", id, why);
                Ok(())
            }
        }
    }

    /// Runs a single step of the server, processing incoming packets.
    #[inline]
    pub fn run_step(&mut self) -> Result<Vec<Packet>> {
        // Process all incoming packets until none remain.
        let mut out = vec![];
        while self.get_packet(&mut out)?.is_some() {}
        self.socket.run_tasks(false).map_err(AppError::Net)?;

        Ok(out)
    }

    /// Processes incoming packets and handles their types.
    fn get_packet(&mut self, out: &mut Vec<Packet>) -> Result<Option<()>> {
        // Receive the packet from the socket.
        let packet = match self.socket.try_recv() {
            Ok(Some(packet)) => packet,
            Ok(None) | Err(NetError::InvalidPacket(..) | NetError::NothingToDo) => return Ok(None),
            Err(NetError::SocketError(why)) => Err(AppError::Net(NetError::SocketError(why)))?,
            Err(why) => {
                debugln!("SERVER: Failed to receive packet: {}", why);
                return Ok(None);
            }
        };

        if let PacketLabel::Disconnect = packet.label() {
            debugln!("SERVER: Client [{}] is disconnecting.", packet.source(),);
            self.disconnect_client(packet.source(), false)?;
            if !self.socket.is_remote() {
                // Local sockets shut the server down on disconnect.
                flee!(AppError::Net(NetError::Disconnected));
            }
        }

        out.push(packet);
        Ok(Some(()))
    }
}

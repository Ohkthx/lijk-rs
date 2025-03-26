use ::std::collections::HashMap;
use ::std::time::{Duration, Instant, SystemTime};

use crate::error::{AppError, Result};
use crate::net::{Deliverable, EntityId, NetError, Packet, PacketLabel, Socket};
use crate::payload::Payload;
use crate::{debugln, flee, utils};

/// Basic server implementation that can handle multiple clients.
pub struct Server {
    socket: Socket,                         // The socket used for communication.
    heartbeats: HashMap<EntityId, Instant>, // The last heartbeat received from each client.

    send_heartbeat: utils::Task, // The interval for sending heartbeats to clients.
    check_heartbeat: utils::Task, // The interval for checking client heartbeats.
}

impl Server {
    /// Maximum time in milliseconds to wait for a heartbeat before disconnecting a client.
    const DISCONNECT_DELTA_MS: u128 = 12000;

    /// Creates a new server with the given connection.
    pub fn new(connection: Socket) -> Self {
        Self {
            socket: connection,
            heartbeats: HashMap::new(),

            send_heartbeat: utils::Task::start(Duration::from_secs(5), 0),
            check_heartbeat: utils::Task::start(Duration::from_secs(11), 0),
        }
    }

    /// Obtains the ID of the server.
    #[inline]
    fn id(&self) -> EntityId {
        self.socket.id()
    }

    /// Sends a packet to the client.
    fn send(
        &mut self,
        packet_type: PacketLabel,
        dest: EntityId,
        payload: Option<Payload>,
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
    fn disconnect_client(&mut self, id: EntityId, notify: bool) -> Result<()> {
        // Remove the client from the list.
        self.heartbeats.remove(&id);
        match self.socket.disconnect_client(id, notify) {
            Ok(()) => Ok(()),
            Err(NetError::SocketError(why)) => Err(AppError::NetError(NetError::SocketError(why))),
            Err(why) => {
                debugln!("SERVER: Error while disconnecting client [{}]: {}", id, why);
                Ok(())
            }
        }
    }

    /// Duration since the epoch.
    fn since_epoch() -> Duration {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
    }

    /// Sends a heartbeat to all connected clients to check their status.
    fn send_heartbeat(&mut self) {
        for id in self.socket.remote_ids() {
            let payload = Payload::Timestamp(true, Self::since_epoch());
            if let Err(why) = self.send(PacketLabel::Heartbeat, id, Some(payload)) {
                debugln!("SERVER: [{}] Failed to send heartbeat: {}", id, why);
            }
        }
    }

    /// Checks if the heartbeat has been received from each client within the disconnect delta.
    fn check_heartbeat(&mut self) {
        let now = Instant::now();
        let mut disconnect: Vec<EntityId> = vec![];

        // Remove clients from the list if the heartbeat has not been received within the disconnect delta.
        self.heartbeats.retain(|id, last_heartbeat| {
            if now.duration_since(*last_heartbeat).as_millis() > Self::DISCONNECT_DELTA_MS {
                disconnect.push(*id);
                false
            } else {
                true
            }
        });

        // Send a disconnect command to the client if the heartbeat has not been received.
        for id in disconnect {
            debugln!("SERVER: [{}] Heartbeat timeout. Disconnecting client.", id);
            if let Err(why) = self.disconnect_client(id, true) {
                debugln!("SERVER: [{}] Failed to disconnect client: {}", id, why);
            }
        }
    }

    /// Runs a single step of the server, processing incoming packets and sending heartbeats.
    #[inline]
    pub fn run_step(&mut self) -> Result<()> {
        // Process all incoming packets until none remain.
        while self.packet_processor()?.is_some() {}

        if self.send_heartbeat.is_ready() {
            self.send_heartbeat();
            self.send_heartbeat.reset();
        }

        if self.check_heartbeat.is_ready() {
            self.check_heartbeat();
            self.check_heartbeat.reset();
        }

        self.socket.run_tasks();

        Ok(())
    }

    /// Processes incoming packets and handles their types.
    fn packet_processor(&mut self) -> Result<Option<Packet>> {
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
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!("SERVER: [{}] Received error: {}", packet.sender(), payload);
                }
            }

            PacketLabel::Acknowledge => {
                debugln!("SERVER: [{}] Received acknowledge.", packet.sender());
            }

            PacketLabel::Connect => {
                debugln!("SERVER: [{}] Client is connecting.", packet.sender());
                self.heartbeats.insert(packet.sender(), Instant::now());
                let payload = Payload::U16(packet.sender());
                self.send(PacketLabel::Connect, packet.sender(), Some(payload))?;
            }

            PacketLabel::Disconnect => {
                debugln!("SERVER: Client [{}] is disconnecting.", packet.sender(),);
                self.disconnect_client(packet.sender(), false)?;
                if !self.socket.is_remote() {
                    // Local sockets shut the server down on disconnect.
                    flee!(AppError::NetError(NetError::Disconnected));
                }
            }

            PacketLabel::Heartbeat => {
                if let Some(ts) = self.heartbeats.get_mut(&packet.sender()) {
                    *ts = Instant::now();
                } else {
                    debugln!(
                        "SERVER: [{}] Client should be disconnected.",
                        packet.sender()
                    );
                    self.disconnect_client(packet.sender(), true)?;
                    return Ok(None);
                }

                let ping = if let Payload::Timestamp(respond, ts) = Payload::from(&packet) {
                    if respond {
                        let payload = Payload::Timestamp(false, Self::since_epoch());
                        self.send(PacketLabel::Heartbeat, packet.sender(), Some(payload))?;
                    }
                    let total = Self::since_epoch() - ts;
                    format!(", ping: {total:?}")
                } else {
                    String::new()
                };

                debugln!("SERVER: [{}] Received heartbeat{}", packet.sender(), ping,);
            }

            PacketLabel::Message => {
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!(
                        "SERVER: [{}] Received message: {}",
                        packet.sender(),
                        payload
                    );
                }
            }

            PacketLabel::Unknown => {
                debugln!(
                    "SERVER: [{}] Received unknown packet label: {:?}.",
                    packet.sender(),
                    packet.label()
                );
            }
        }

        Ok(Some(packet))
    }
}

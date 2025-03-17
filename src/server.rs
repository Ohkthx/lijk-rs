use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime},
};

use anyhow::{Result, bail};
use uuid::Uuid;

use crate::{debugln, utils};
use crate::{net::ConnectionError, payload::Payload};
use crate::{
    net::{Deliverable, Packet, PacketType, Socket},
    payload::Timestamp,
};

/// Basic server implementation that can handle multiple clients.
pub struct Server {
    socket: Socket,                     // The socket used for communication.
    heartbeats: HashMap<Uuid, Instant>, // The last heartbeat received from each client.

    send_heartbeat: utils::Interval, // The interval for sending heartbeats to clients.
    check_heartbeat: utils::Interval, // The interval for checking client heartbeats.
}

impl Server {
    /// Maximum time in milliseconds to wait for a heartbeat before disconnecting a client.
    const DISCONNECT_DELTA_MS: u128 = 12000;

    /// Creates a new server with the given connection.
    pub fn new(connection: Socket) -> Self {
        Self {
            socket: connection,
            heartbeats: HashMap::new(),

            send_heartbeat: utils::Interval::start(Duration::from_secs(5), 0),
            check_heartbeat: utils::Interval::start(Duration::from_secs(10), 0),
        }
    }

    /// Obtains the UUID of the server.
    #[inline]
    fn uuid(&self) -> Uuid {
        self.socket.uuid()
    }

    /// Sends a packet to the client.
    fn send(&mut self, packet_type: PacketType, dest: Uuid, payload: Option<&[u8]>) -> Result<()> {
        if self.uuid() == dest {
            bail!(ConnectionError::SelfConnection);
        }

        let mut packet = Packet::new(packet_type, self.uuid());
        if let Some(data) = payload {
            packet.set_payload(data);
        }

        self.socket.send(Deliverable::new(dest, packet))
    }

    /// Disconnects a client from the server and removes it from the list.
    fn disconnect_client(&mut self, uuid: Uuid, notify: bool) -> Result<()> {
        // Remove the client from the list.
        self.heartbeats.remove(&uuid);
        self.socket.disconnect_client(uuid, notify)?;
        Ok(())
    }

    /// Duration since the epoch.
    fn since_epoch() -> Duration {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
    }

    /// Sends a heartbeat to all connected clients to check their status.
    fn send_heartbeat(&mut self) {
        for uuid in self.socket.remote_uuids() {
            let now = Timestamp(
                Self::since_epoch().as_secs(),
                Self::since_epoch().subsec_nanos(),
            );

            let payload = std::convert::Into::<Vec<u8>>::into(now);
            if let Err(why) = self.send(PacketType::Heartbeat, uuid, Some(&payload)) {
                debugln!(
                    "SERVER: [{}] Failed to send heartbeat: {}",
                    uuid.as_fields().0,
                    why
                );
            }
        }
    }

    /// Checks if the heartbeat has been received from each client within the disconnect delta.
    fn check_heartbeat(&mut self) {
        let now = Instant::now();
        let mut disconnect: Vec<Uuid> = vec![];

        // Remove clients from the list if the heartbeat has not been received within the disconnect delta.
        self.heartbeats.retain(|uuid, last_heartbeat| {
            if now.duration_since(*last_heartbeat).as_millis() > Self::DISCONNECT_DELTA_MS {
                disconnect.push(*uuid);
                false
            } else {
                true
            }
        });

        // Send a disconnect command to the client if the heartbeat has not been received.
        for uuid in disconnect {
            debugln!(
                "SERVER: [{}] Heartbeat timeout. Disconnecting client.",
                uuid.as_fields().0
            );
            if let Err(why) = self.disconnect_client(uuid, true) {
                debugln!(
                    "SERVER: [{}] Failed to disconnect client: {}",
                    uuid.as_fields().0,
                    why
                );
            }
        }
    }

    /// Runs a single step of the server, processing incoming packets and sending heartbeats.
    #[inline]
    pub fn run_step(&mut self) -> Result<()> {
        self.packet_processor()?;

        if self.send_heartbeat.is_ready() {
            self.send_heartbeat();
            self.send_heartbeat.reset();
        }

        if self.check_heartbeat.is_ready() {
            self.check_heartbeat();
            self.check_heartbeat.reset();
        }

        Ok(())
    }

    /// Runs the server and handles incoming packets.
    pub fn run(&mut self) -> Result<()> {
        loop {
            self.run_step()?;
        }
    }

    /// Processes incoming packets and handles their types.
    fn packet_processor(&mut self) -> Result<()> {
        let Some(packet) = self.socket.try_recv()? else {
            return Ok(());
        };

        match packet.get_type() {
            PacketType::Error => {
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!(
                        "SERVER: [{}] Received error: {}",
                        packet.get_short_id(),
                        payload
                    );
                }
            }

            PacketType::Acknowledge => {
                debugln!("SERVER: [{}] Received acknowledge.", packet.get_short_id());
            }

            PacketType::Connect => {
                debugln!("SERVER: [{}] Client is connecting.", packet.get_short_id());
                self.heartbeats.insert(packet.get_source(), Instant::now());
                let payload = Payload::Uuid(packet.get_source()).to_bytes();
                self.send(PacketType::Connect, packet.get_source(), Some(&payload))?;
            }

            PacketType::Disconnect => {
                debugln!(
                    "SERVER: Client [{}] is disconnecting.",
                    packet.get_short_id(),
                );
                self.disconnect_client(packet.get_source(), false)?;
                if self.socket.is_local() {
                    // Local sockets shut the server down on disconnect.
                    bail!(ConnectionError::Disconnected);
                }
            }

            PacketType::Heartbeat => {
                if let Some(ts) = self.heartbeats.get_mut(&packet.get_source()) {
                    *ts = Instant::now();
                } else {
                    debugln!(
                        "SERVER: [{}] Client should be disconnected.",
                        packet.get_short_id()
                    );
                    self.disconnect_client(packet.get_source(), true)?;
                    return Ok(());
                }

                let ping = if let Payload::Timestamp(ts) = Payload::from(&packet) {
                    let total = Self::since_epoch() - ts;
                    format!(", ping: {total:?}")
                } else {
                    String::new()
                };

                debugln!(
                    "SERVER: [{}] Received heartbeat{}",
                    packet.get_short_id(),
                    ping,
                );
            }

            PacketType::Message => {
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!(
                        "SERVER: [{}] Received message: {}",
                        packet.get_short_id(),
                        payload
                    );
                }
            }
        }

        Ok(())
    }
}

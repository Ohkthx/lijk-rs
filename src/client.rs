use std::time::{Duration, SystemTime};

use anyhow::{Result, bail};
use uuid::Uuid;

use crate::debugln;
use crate::net::{ConnectionError, Deliverable, Packet, PacketType, Socket};
use crate::payload::Payload;

/// Basic client implementation that connects to a server.
pub struct Client {
    socket: Socket,             // The socket used for communication.
    server: Uuid,               // The UUID of the server to connect to.
    server_ts_offset: Duration, // The offset between the server and client timestamps.
}

impl Client {
    /// Maximum number of connection retries before disconnecting.
    const MAX_CONNECTION_RETRY: u8 = 30;

    /// Creates a new client with the given connection.
    pub fn new(connection: Socket) -> Self {
        Self {
            socket: connection,
            server: Uuid::nil(),
            server_ts_offset: Duration::from_secs(0),
        }
    }

    /// Obtains the UUID of the client.
    #[inline]
    fn uuid(&self) -> Uuid {
        self.socket.uuid()
    }

    /// Client ID.
    fn client_id(&self) -> String {
        self.uuid().as_fields().0.to_string()
    }

    /// Sends a packet to the server.
    fn send(&mut self, packet_type: PacketType, payload: Option<&[u8]>) -> Result<()> {
        let mut packet = Packet::new(packet_type, self.uuid());

        if let Some(data) = payload {
            packet.set_payload(data);
        }

        self.socket.send(Deliverable::new(self.server, packet))?;
        Ok(())
    }

    /// Duration since the epoch.
    fn since_epoch() -> Duration {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
    }

    /// Runs the client and handles incoming packets.
    pub fn run(&mut self) -> Result<()> {
        let mut retry_count = 0;
        while retry_count < Self::MAX_CONNECTION_RETRY && self.server.is_nil() {
            // Send a connect packet to the server.
            self.send(PacketType::Connect, None)?;
            std::thread::sleep(Duration::from_millis(500));

            self.packet_processor()?;
            retry_count += 1;
        }

        // Check if a connection was never established.
        if retry_count >= Self::MAX_CONNECTION_RETRY {
            bail!(ConnectionError::Timeout);
        } else if self.server.is_nil() {
            bail!(ConnectionError::NotConnected);
        }

        loop {
            self.packet_processor()?;
        }
    }

    /// Processes incoming packets and handles different packet types.
    fn packet_processor(&mut self) -> Result<()> {
        let Some(packet) = self.socket.try_recv()? else {
            return Ok(());
        };

        match packet.get_type() {
            PacketType::Error => {
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!("CLIENT: [{}] Received error: {}", self.client_id(), payload);
                }
            }

            PacketType::Acknowledge => {
                debugln!("CLIENT: [{}] Received acknowledge.", self.client_id());
            }

            PacketType::Connect => {
                self.server = packet.get_source();
                debugln!(
                    "CLIENT: [{}] Connected, Server: {}.",
                    self.client_id(),
                    self.server.as_fields().0
                );
            }

            PacketType::Disconnect => {
                debugln!(
                    "CLIENT: [{}] Server sent disconnect command.",
                    self.client_id()
                );

                if self.socket.is_local() {
                    // Notify server for safe shutdown on local sockets.
                    self.send(PacketType::Disconnect, None)?;
                }

                bail!(ConnectionError::Disconnected);
            }

            PacketType::Heartbeat => {
                let Payload::Timestamp(duration) = Payload::from(&packet) else {
                    bail!(ConnectionError::InvalidPacketPayload(
                        "Timestamp for Heartbeat (Missing)".to_string()
                    ));
                };

                self.server_ts_offset = Self::since_epoch() - duration;
                debugln!(
                    "CLIENT: [{}] Received heartbeat, ping: {:?}",
                    self.client_id(),
                    self.server_ts_offset
                );

                let payload = Payload::Timestamp(duration).to_bytes();
                let _ = self.send(PacketType::Heartbeat, Some(&payload));
            }

            PacketType::Message => {
                if let Payload::String(payload) = Payload::from(packet) {
                    debugln!(
                        "CLIENT: [{}] Received message: {}",
                        self.client_id(),
                        payload
                    );
                }
            }
        }

        Ok(())
    }
}

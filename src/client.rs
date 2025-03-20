use std::time::{Duration, Instant, SystemTime};

use anyhow::{Result, bail};

use crate::net::{ConnectionError, Deliverable, Packet, PacketType, Socket};
use crate::payload::Payload;
use crate::{debugln, utils};

/// Basic client implementation that connects to a server.
pub struct Client {
    socket: Socket,             // The socket used for communication.
    server: u32,                // The UUID of the server to connect to.
    server_ts_offset: Duration, // The offset between the server and client timestamps.

    last_packet_ts: Instant,         // The last time a packet was received.
    send_heartbeat: utils::Interval, // The interval for sending heartbeats to the server.
    check_timeout: utils::Interval,  // The interval for checking if the client timed out.
}

impl Client {
    /// Maximum number of connection retries before disconnecting.
    const MAX_CONNECTION_RETRY: u8 = 30;
    /// Amount of time to check for a heartbeat before disconnecting.
    const RECONNECT_DELTA_MS: u128 = 5000;
    /// Amount of time from last heartbeat to disconnect.
    const TIMEOUT_DELTA_MS: u128 = 21000;

    /// Creates a new client with the given connection.
    pub fn new(connection: Socket) -> Self {
        Self {
            socket: connection,
            server: Socket::INVALID_CLIENT_ID,
            server_ts_offset: Duration::from_secs(0),

            last_packet_ts: Instant::now(),
            send_heartbeat: utils::Interval::start(Duration::from_secs(10), 0),
            check_timeout: utils::Interval::start(Duration::from_secs(5), 0),
        }
    }

    /// Obtains the UUID of the client.
    #[inline]
    fn id(&self) -> u32 {
        self.socket.id()
    }

    /// Sends a packet to the server.
    fn send(&mut self, packet_type: PacketType, payload: Option<&[u8]>) -> Result<()> {
        let mut packet = Packet::new(packet_type, self.id());

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

    /// Sends a heartbeat to the server to ensure the client is still connected.
    fn send_heartbeat(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_packet_ts).as_millis() > Self::RECONNECT_DELTA_MS {
            debugln!("CLIENT: [{}] Checking if server alive.", self.id());
            let payload = Payload::Timestamp(true, Self::since_epoch()).as_bytes();
            if let Err(why) = self.send(PacketType::Heartbeat, Some(&payload)) {
                debugln!("CLIENT: [{}] Failed to send heartbeat: {}", self.id(), why);
            }
        }
    }

    /// Check if the client has timed out due not receiving a packet in time.
    fn check_timeout(&mut self) -> Result<()> {
        let now = Instant::now();
        if now.duration_since(self.last_packet_ts).as_millis() > Self::TIMEOUT_DELTA_MS {
            bail!(ConnectionError::Timeout);
        }

        Ok(())
    }

    /// Waits for a connection to be established with the server.
    pub fn wait_for_connection(&mut self) -> Result<()> {
        let mut retry_count = 0;
        while retry_count < Self::MAX_CONNECTION_RETRY && self.server == Socket::INVALID_CLIENT_ID {
            // Send a connect packet to the server.
            self.send(PacketType::Connect, None)?;
            std::thread::sleep(Duration::from_millis(500));

            self.packet_processor()?;
            retry_count += 1;
        }

        // Check if a connection was never established.
        if retry_count >= Self::MAX_CONNECTION_RETRY {
            bail!(ConnectionError::Timeout);
        } else if self.server == Socket::INVALID_CLIENT_ID {
            bail!(ConnectionError::NotConnected);
        }

        Ok(())
    }

    /// Runs a single step of the client, processing packets and handling timeouts.
    pub fn run_step(&mut self) -> Result<()> {
        while self.packet_processor()?.is_some() {}

        if self.send_heartbeat.is_ready() {
            self.send_heartbeat();
            self.send_heartbeat.reset();
        }

        if self.check_timeout.is_ready() {
            self.check_timeout()?;
            self.check_timeout.reset();
        }
        Ok(())
    }

    /// Processes incoming packets and handles different packet types.
    fn packet_processor(&mut self) -> Result<Option<Packet>> {
        let Some(packet) = self.socket.try_recv()? else {
            return Ok(None);
        };

        // Update the last packet received timestamp.
        self.last_packet_ts = Instant::now();

        match packet.get_type() {
            PacketType::Error => {
                if let Payload::Error(code, Some(msg)) = Payload::from(&packet) {
                    debugln!("CLIENT: [{}] Received error [{}]: {}", self.id(), code, msg);
                }
            }

            PacketType::Acknowledge => {
                debugln!("CLIENT: [{}] Received acknowledge.", self.id());
            }

            PacketType::Connect => {
                self.server = packet.get_sender();
                debugln!(
                    "CLIENT: [{}] Connected, Server: {}.",
                    self.id(),
                    self.server
                );
            }

            PacketType::Disconnect => {
                debugln!("CLIENT: [{}] Server sent disconnect command.", self.id());

                if self.socket.is_local() {
                    // Notify server for safe shutdown on local sockets.
                    self.send(PacketType::Disconnect, None)?;
                }

                bail!(ConnectionError::Disconnected);
            }

            PacketType::Heartbeat => {
                let Payload::Timestamp(respond, duration) = Payload::from(&packet) else {
                    bail!(ConnectionError::InvalidPacketPayload(
                        "Timestamp for Heartbeat (Missing)".to_string()
                    ));
                };

                self.server_ts_offset = Self::since_epoch() - duration;
                debugln!(
                    "CLIENT: [{}] Received heartbeat, ping: {:?}",
                    self.id(),
                    self.server_ts_offset
                );

                if respond {
                    let payload = Payload::Timestamp(false, duration).as_bytes();
                    let _ = self.send(PacketType::Heartbeat, Some(&payload));
                }
            }

            PacketType::Message => {
                if let Payload::String(payload) = Payload::from(&packet) {
                    debugln!("CLIENT: [{}] Received message: {}", self.id(), payload);
                }
            }
        }

        Ok(Some(packet))
    }
}

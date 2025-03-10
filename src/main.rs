use anyhow::{Result, bail};
use connection::{Connection, ConnectionError, Packet, PacketHandler, PacketType};
use uuid::Uuid;

mod connection;

/// Exmaple of a payload from a packet.
enum Payload {
    None,
    String(String),
    Uuid(Uuid),
}

impl From<&Packet> for Payload {
    fn from(value: &Packet) -> Self {
        let raw = value.get_payload();
        if raw.is_empty() {
            return Self::None;
        }

        match value.get_type() {
            PacketType::Error | PacketType::Message => {
                Self::String(String::from_utf8_lossy(raw).to_string())
            }
            PacketType::Connect => Self::Uuid(Uuid::from_slice(raw).unwrap_or_default()),
            _ => Self::None,
        }
    }
}

impl From<Packet> for Payload {
    fn from(value: Packet) -> Self {
        Self::from(&value)
    }
}

/// Basic server implementation with 1 client.
struct Server {
    connection: Connection,
    uuid: Uuid,
}

impl Server {
    /// Creates a new server with the given connection.
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            uuid: Uuid::new_v4(),
        }
    }

    /// Sends a packet to the client.
    pub fn send(&self, packet: Packet) {
        self.connection.send(packet);
    }

    /// Runs the server and handles incoming packets.
    fn run(&mut self) -> Result<()> {
        println!("Server is running!");
        const MAX_PINGS: usize = 5;
        let mut pings = 0;

        loop {
            let packet = self.connection.recv()?;
            match packet.get_type() {
                PacketType::Error => {
                    if let Payload::String(payload) = Payload::from(packet) {
                        println!("SERVER: Received error: {}", payload);
                    }
                }

                PacketType::Acknowledge => println!("SERVER: Received acknowledge."),

                PacketType::Connect => {
                    println!("SERVER: Client is connecting.");
                    let mut packet = Packet::new(PacketType::Connect, self.uuid);
                    packet.set_payload(Uuid::new_v4().as_bytes());
                    self.send(packet);
                }

                PacketType::Disconnect => {
                    println!("SERVER: Client [{}] is disconnecting.", packet.get_uuid());
                    break;
                }

                PacketType::Ping => {
                    println!("SERVER: Received Ping.");
                    pings += 1;
                    if pings >= MAX_PINGS {
                        println!("SERVER: Max pings reached. Sending disconnect command.");
                        self.send(Packet::new(PacketType::Disconnect, Uuid::new_v4()));
                        break;
                    }
                    self.send(Packet::new(PacketType::Pong, Uuid::new_v4()));
                }

                PacketType::Pong => {
                    println!("SERVER: Received Pong.");
                    pings += 1;
                    if pings >= MAX_PINGS {
                        println!("SERVER: Max pings reached. Sending disconnect command.");
                        self.send(Packet::new(PacketType::Disconnect, Uuid::new_v4()));
                        break;
                    }
                    self.send(Packet::new(PacketType::Ping, Uuid::new_v4()));
                }

                PacketType::Message => {
                    if let Payload::String(payload) = Payload::from(packet) {
                        println!("SERVER: Received message: {}", payload);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Basic client implementation that connects to a server.
struct Client {
    connection: Connection,
    server: Uuid,
    uuid: Uuid,
}

impl Client {
    /// Creates a new client with the given connection.
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            server: Uuid::nil(),
            uuid: Uuid::nil(),
        }
    }

    /// Sends a packet to the server.
    pub fn send(&self, packet: Packet) {
        self.connection.send(packet);
    }

    /// Runs the client and handles incoming packets.
    fn run(&mut self) -> Result<()> {
        // Send a connect packet to the server.
        self.send(Packet::new(PacketType::Connect, Uuid::nil()));

        loop {
            let packet = self.connection.recv()?;
            match packet.get_type() {
                PacketType::Error => {
                    if let Payload::String(payload) = Payload::from(&packet) {
                        println!("CLIENT: Received error: {}", payload);
                    }
                }

                PacketType::Acknowledge => println!("CLIENT: Received acknowledge."),

                PacketType::Connect => {
                    if let Payload::Uuid(payload) = Payload::from(&packet) {
                        self.uuid = payload;
                    } else {
                        println!("CLIENT: Received invalid connect packet.");
                        bail!(ConnectionError::AuthenticationFailed);
                    }

                    self.server = packet.get_uuid();
                    println!("CLIENT: Connected, UUID: {}.", self.uuid);
                    self.send(Packet::new(PacketType::Ping, self.uuid));
                }

                PacketType::Disconnect => {
                    println!("CLIENT: Server sent disconnect command.");
                    break;
                }

                PacketType::Ping => {
                    println!("CLIENT: Received Ping.");
                    self.send(Packet::new(PacketType::Pong, self.uuid));
                }

                PacketType::Pong => {
                    println!("CLIENT: Received Pong.");
                    self.send(Packet::new(PacketType::Ping, self.uuid));
                }

                PacketType::Message => {
                    if let Payload::String(payload) = Payload::from(packet) {
                        println!("CLIENT: Received message: {}", payload);
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    // Initialize the local connections.
    let (server_connection, client_connection) = Connection::new_local()?;

    // Spawn the server with a connection in a separate thread.
    let server_run = std::thread::spawn(move || {
        let mut server = Server::new(server_connection);
        server.run()
    });

    // Create the client with a connection.
    let mut client = Client::new(client_connection);
    let result = client.run();
    let _ = server_run.join();

    result
}

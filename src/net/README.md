# LIJK::NET

**LIJK::NET** abstracts both MPSC and UDP Sockets into a singular `Socket` type that provides a high-level interface for communication between remote clients and hosts.

## Overview

This module offers two primary types of sockets:

- **`LocalSocket`**: A Multi-Producer, Single-Consumer (MPSC) channel for fast interprocess communication within the same application.
- **`RemoteSocket`**: A UDP-based socket designed for efficient communication over a network.

Both of these are unified under the **`Socket`** abstraction, allowing users to seamlessly work with different types of connections without worrying about the underlying implementation.

---

## Components

### `LocalSocket` - MPSC Communication

`LocalSocket` facilitates message passing between components within the same process. This is ideal for client-server communication within a single application due to:

- Low overhead
- High-speed message transmission
- No reliance on external networking layers

#### Example Usage

```rust
let (mut server, mut client) = Socket::new_local_pair().expect("Failed to create local socket pair");
```

---

### `RemoteSocket` - UDP-Based Communication

`RemoteSocket` is a UDP-based implementation that provides a connectionless method for sending and receiving data across networks. Unlike TCP, it does not guarantee packet delivery, but it offers lower latency and improved speed for certain use cases.

- **Packet Identification**: Each packet contains a `sequence_id` to track the order of delivery.
- **Supports Multiple Clients**: Allows servers to handle multiple remote connections efficiently.

#### Example Usage

```rust
let mut remote_socket = Socket::new_remote(Some("127.0.0.1:4000".to_string())).expect("Failed to create remote socket");
```

---

### `Socket` - Unified Interface

The `Socket` struct abstracts both `LocalSocket` and `RemoteSocket`, providing a consistent API for handling communication. The user does not need to worry about the underlying implementation details.

#### Key Features

- Unified API for both local and remote communication.
- Server and client modes are seamlessly handled.
- Automatic packet validation and sequence tracking.

#### Example Usage

```rust
let mut socket = Socket::new_remote(None).expect("Failed to create server socket");
if socket.is_server() {
    println!("Socket is running in server mode on {}", socket.address());
}
```

---

## Packet Structure

Communication within `LIJK::NET` is built around `Packet` structs, which encapsulate data sent between clients and servers. Each packet consists of:

- **Version (`u8`)**: Indicates the protocol version.
- **Type (`PacketType`)**: Defines the purpose of the packet (e.g., `Connect`, `Disconnect`, `Message`).
- **Source (`Uuid`)**: The unique identifier of the sender.
- **Sequence (`u32`)**: A counter used to track packet order.
- **Payload (`Vec<u8>`)**: The actual message contents.

### `PacketType` Enumeration

```rust
enum PacketType {
    Error = 0x00,
    Acknowledge,
    Connect,
    Disconnect,
    Heartbeat,
    Message,
}
```

The `Packet` struct ensures that each transmission maintains integrity, allowing clients and servers to validate and process incoming data efficiently.

#### Example Usage

```rust
let packet = Packet::new(PacketType::Message, socket.uuid());
packet.set_payload(b"Hello, world!");

socket.send(Deliverable::new(recipient_uuid, packet)).expect("Failed to send packet");
```

---

## Error Handling

The module defines a set of `ConnectionError` types to manage various connection-related issues. These include:

- `DuplicateConnection`: Connection already exists.
- `AuthenticationFailed`: Failed to authenticate the connection.
- `Timeout`: Connection attempt timed out.
- `SelfConnection`: Attempting to connect to self is not allowed.
- `InvalidPacket*`: Various errors related to malformed packets.

All errors implement `std::error::Error` for ease of handling.

```rust
match socket.send(deliverable) {
    Ok(_) => println!("Packet sent successfully"),
    Err(e) => eprintln!("Failed to send packet: {}", e),
}
```

---

## Conclusion

LIJK::NET provides a high-level, efficient, and flexible abstraction for both interprocess and network communication. Whether communicating within a single application or across a network, this module offers a streamlined API that simplifies socket management while maintaining performance and reliability.

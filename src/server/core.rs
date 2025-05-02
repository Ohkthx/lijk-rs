use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::AppError;
use crate::net::{Packet, PacketLabel, Socket};
use crate::shared::payload::{
    Connect, Movement, PayloadId, Position as PositionPayload, ServerState,
};
use crate::utils::{Timestep, decode};
use crate::vec2f::Vec2f;

use super::ClientEntityMap;
use super::components::Position;
use super::ecs::World;
use super::socket::ServerSocket;
use super::sys;
use super::world_map::WorldMap;

/// Core of the server loop.
pub struct ServerCore {
    socket: ServerSocket,            // Socket for network communication.
    sigint: Option<Arc<AtomicBool>>, // Optional signal interrupt handler.
}

impl ServerCore {
    /// Creates a new `ServerCore` instance with the given socket and optional signal interrupt handler.
    pub fn new(socket: Socket, sigint: Option<Arc<AtomicBool>>) -> Self {
        Self {
            socket: ServerSocket::new(socket),
            sigint,
        }
    }

    /// Runs the main server loop. Processes incoming packets and updates the game state.
    pub fn run(&mut self, ticks_per_second: u16) -> Result<(), AppError> {
        let mut step = Timestep::new(f32::from(ticks_per_second));

        // Allows for bi-directional mapping between clients and entities.
        let mut client_entity = ClientEntityMap::new();

        // Create a new world instance to manage entities and components.
        let mut world = World::new();
        world.register_component::<Position>();
        world.register_component::<Movement>();

        let world_map = WorldMap::new(Vec2f::ZERO, 100.0, 100.0);

        'core_loop: loop {
            // Ensure a kill command has not been sent.
            if let Some(sigint) = &self.sigint {
                if sigint.load(Ordering::Relaxed) {
                    break 'core_loop;
                }
            }

            // Send the server state to all clients at the specified tick rate.
            if step.tick() % u64::from(ticks_per_second) == 0 {
                for client in client_entity.iter_clients() {
                    // Send the server state to the client.
                    let mut to_send = Packet::new(
                        PacketLabel::Extension(u8::from(PayloadId::State)),
                        self.socket.id(),
                    );

                    to_send.set_payload(ServerState {
                        tps: ticks_per_second,
                        tick_id: step.tick(),
                    });

                    self.socket.send(*client, to_send)?;
                }
            }

            let packets = self.socket.run_step()?;
            for packet in packets {
                match packet.label() {
                    PacketLabel::Connect => {
                        println!("Client connected: {}", packet.source());

                        // Spawn a new entity for the client.
                        let entity = world.spawn_entity().build();
                        world.attach_component(entity, Position(Vec2f::ZERO));
                        client_entity.add(packet.source(), entity);

                        // Send initial position to the client.
                        let mut to_send = Packet::new(
                            PacketLabel::Extension(u8::from(PayloadId::Connect)),
                            packet.source(),
                        );
                        to_send.set_payload(Connect(u32::from(entity), *world_map.spawn_point()));
                        self.socket.send(packet.source(), to_send)?;
                    }

                    PacketLabel::Extension(id) if id == u8::from(PayloadId::Movement) => {
                        let payload = decode::<Movement>(&packet)?;
                        if let Some(entity) = client_entity.get_entity(packet.source()) {
                            if payload.0 == Vec2f::ZERO {
                                // If the movement vector is zero, remove the Movement component.
                                world.detach_component::<Movement>(entity);
                            } else {
                                // Otherwise, attach or update the Movement component.
                                world.attach_component(entity, payload);
                            }
                        }
                    }

                    _ => {}
                }
            }

            let label = PacketLabel::Extension(u8::from(PayloadId::Position));
            let changes = sys::movement(&mut world, &world_map, step.fixed_dt());
            world.fetch_components(|entity, pos: &Position| {
                if !changes.contains(&entity) {
                    return;
                }

                for client in client_entity.iter_clients() {
                    // Send the updated position to all clients.
                    let mut to_send = Packet::new(label, self.socket.id());
                    to_send.set_payload(PositionPayload(u32::from(entity), pos.0));
                    self.socket.send(*client, to_send).unwrap();
                }
            });

            step.wait();
        }

        Ok(())
    }
}

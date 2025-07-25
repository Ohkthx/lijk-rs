use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::AppError;
use crate::net::{Packet, PacketLabel, Socket};
use crate::server::ai::AiState;
use crate::shared::payload::{
    Connect, Movement, PayloadId, Position as PositionPayload, ServerState,
};
use crate::shared::shape::Rectangle;
use crate::shared::transform::Transform;
use crate::utils::{SpatialHash, Timestep, decode};
use crate::vec2f::Vec2f;

use super::ClientEntityMap;
use super::ai::BasicAi;
use super::ecs::{Entity, World};
use super::socket::ServerSocket;
use super::spawner::{Owner, Spawner};
use super::sys;
use super::world_map::WorldMap;

struct Name(pub String);
pub(crate) struct LastTarget(pub Option<Entity>);

pub(crate) struct Slime;
impl Slime {
    pub fn spawn(world: &mut World, pos: Vec2f) -> Entity {
        world
            .spawn_entity()
            .attach(Name("a Slime".to_string()))
            .attach(Transform::with_position(pos))
            .attach(Rectangle::new(1.0, 1.0))
            .attach(Movement(Vec2f::ZERO, 1))
            .attach(BasicAi::new())
            .attach(LastTarget(None))
            .build()
    }
}

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

        // Spatial hash for tracking entitiy positions and detecting collisions.
        let mut gps = SpatialHash::new(1.0);

        // Create a new world instance to manage entities and components.
        let mut world = World::new();
        world.register_component::<Transform>();
        world.register_component::<Rectangle>();
        world.register_component::<Movement>();
        world.register_component::<Owner>();
        world.register_component::<BasicAi>();
        world.register_component::<LastTarget>();
        world.register_component::<Name>();
        world.register_component::<Spawner>();

        let world_map = WorldMap::new(Vec2f(10.0, 10.0), 18.0, 18.0);

        // Create a spawner to generate test entities.
        // world
        //     .spawn_entity()
        //     .attach(Spawner::new(20, 5.0, 0.5))
        //     .attach(Position(*world_map.spawn_point()))
        //     .build();

        let slime = Slime::spawn(&mut world, *world_map.spawn_point() + Vec2f(10.0, 10.0));
        gps.insert(*world_map.spawn_point(), slime.into());

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

            // Process all incoming packets.
            let packets = self.socket.run_step()?;
            for packet in packets {
                match packet.label() {
                    PacketLabel::Connect => {
                        println!("Client connected: {}", packet.source());

                        // Spawn a new entity for the client.
                        let entity = world.spawn_entity().build();
                        world.attach_component(entity, Rectangle::new(1.0, 1.0));
                        world.attach_component(
                            entity,
                            Transform::with_position(*world_map.spawn_point()),
                        );
                        client_entity.add(packet.source(), entity);

                        // Make the slime follow the player.
                        if let Some(mut ai) = world.fetch_component::<&mut BasicAi>(slime) {
                            world.attach_component(slime, LastTarget(Some(entity)));
                            ai.set_state(AiState::Pursue);
                        }

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
                            world.attach_component(entity, payload);
                        }
                    }

                    _ => {}
                }
            }

            // Trigger a run on the systems.
            let label = PacketLabel::Extension(u8::from(PayloadId::Position));
            sys::ai(&mut world);
            let mut changes = sys::movement(&mut world, &world_map, &mut gps, step.fixed_dt());
            changes.extend(sys::spawn(&mut world, &world_map));

            // Send new positions to the clients.
            world.fetch_components(|entity, transform: &Transform, movement: &Movement| {
                for client in client_entity.iter_clients() {
                    // Send the updated position to all clients.
                    let mut to_send = Packet::new(label, self.socket.id());
                    to_send.set_payload(PositionPayload(
                        u32::from(entity),
                        transform.position,
                        movement.0,
                    ));
                    self.socket.send(*client, to_send).unwrap();
                }
            });

            step.wait();
        }

        Ok(())
    }
}

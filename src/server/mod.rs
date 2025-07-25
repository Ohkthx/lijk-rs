#![allow(dead_code)]

mod ai;
mod core;
mod ecs;
mod socket;
mod spawner;
mod sys;
mod world_map;

pub use core::ServerCore;
use std::collections::HashMap;

use ecs::Entity;

use crate::net::ClientId;

/// A mapping between client IDs and entities.
struct ClientEntityMap {
    client_entity: HashMap<ClientId, Entity>, // client_id -> entity
    entity_client: HashMap<Entity, ClientId>, // entity -> client_id
}

impl ClientEntityMap {
    /// Creates a new `ClientEntityMap`.
    fn new() -> Self {
        Self {
            client_entity: HashMap::new(),
            entity_client: HashMap::new(),
        }
    }

    /// Adds a client ID and entity to the map.
    fn add(&mut self, client_id: ClientId, entity: Entity) {
        self.client_entity.insert(client_id, entity);
        self.entity_client.insert(entity, client_id);
    }

    /// Removes a client ID and entity from the map.
    #[allow(dead_code)]
    fn remove(&mut self, client_id: ClientId) {
        if let Some(entity) = self.client_entity.remove(&client_id) {
            self.entity_client.remove(&entity);
        }
    }

    /// Gets the entity associated with a client ID.
    fn get_entity(&self, client_id: ClientId) -> Option<Entity> {
        self.client_entity.get(&client_id).copied()
    }

    /// Gets the client ID associated with an entity.
    fn get_client(&self, entity: Entity) -> Option<ClientId> {
        self.entity_client.get(&entity).copied()
    }

    /// Iterates over all client IDs.
    fn iter_clients(&self) -> impl Iterator<Item = &ClientId> {
        self.client_entity.keys()
    }
}

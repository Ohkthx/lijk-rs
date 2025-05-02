#![allow(dead_code)]

use std::any::TypeId;
use std::collections::HashSet;

use super::component::{ComponentRef, ComponentStorage};
use super::entity::Entity;
use super::query::Query;
use super::resource::{ResourceRef, ResourceStorage};

/// Command enum to represent actions that can be performed on entities.
pub enum Command {
    Detach(Entity, TypeId), // Detach a component from an entity.
    Kill(Entity),           // Kill an entity.
}

/// `EntityBuilder` struct to facilitate building entities with components.
pub struct EntityBuilder<'a> {
    world: &'a mut World, // Reference to the world.
    entity: Entity,       // The entity being built.
}

impl<'a> EntityBuilder<'a> {
    /// Constructor to create a new `EntityBuilder`.
    pub fn new(world: &'a mut World, entity: Entity) -> Self {
        Self { world, entity }
    }

    /// Method to add a component to the entity.
    pub fn attach<T: 'static>(self, component: T) -> Self {
        self.world.attach_component(self.entity, component);
        self
    }

    /// Used to finalize or error check the entity.
    pub fn build(self) -> Entity {
        self.entity
    }
}

/// World struct to manage entities and their components.
pub struct World {
    /// Contains all components for the world.
    pub(crate) components: ComponentStorage,
    /// Contains all resources for the world.
    pub(crate) resources: ResourceStorage,

    /// Next entity ID to be used.
    next_entity_id: Entity,
    /// Recycled entities for reuse.
    recycled_entities: Vec<Entity>,
}

impl World {
    /// Creates a new instance of the world.
    pub fn new() -> Self {
        Self {
            components: ComponentStorage::default(),
            resources: ResourceStorage::default(),

            next_entity_id: Entity::from(1u32),
            recycled_entities: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Entity management

    /// Generates a new unique entity ID.
    fn generate_id(&mut self) -> Entity {
        let entity = self.next_entity_id;
        self.next_entity_id += 1;
        entity
    }

    /// Creates a new entity in the world.
    pub fn spawn_entity(&mut self) -> EntityBuilder {
        let entity = self
            .recycled_entities
            .pop()
            .map_or_else(|| self.generate_id(), |entity| entity);
        EntityBuilder::new(self, entity)
    }

    /// Kills an entity and removes all its components.
    pub fn kill_entity(&mut self, entity: Entity) {
        // Remove all components associated with the entity.
        self.components.remove_entity(entity);

        // Recycle the entity for future use.
        self.recycled_entities.push(entity);
    }

    /// Retrieves all entities that have a specific component type.
    pub fn fetch_entities<C: 'static>(&self) -> HashSet<Entity> {
        let mut entities = HashSet::new();

        if let Some(set) = self.components.get::<C>() {
            for (entity, _) in set.iter() {
                entities.insert(Entity::from(entity));
            }
        }

        entities
    }

    // -----------------------------------------------------------------------
    // Component management

    /// Registers a component type with the world.
    pub fn register_component<C: 'static>(&mut self) {
        self.components.create::<C>();
    }

    /// Deregisters a component type from the world.
    pub fn deregister_component<C: 'static>(&mut self) {
        self.components.destroy::<C>();
    }

    /// Adds a component to an entity.
    pub fn attach_component<C: 'static>(&self, entity: Entity, component: C) {
        if let Some(mut set) = self.components.get_mut::<C>() {
            set.insert(entity.into(), component);
        } else {
            panic!("No SparseSet found for component type. Did you forget to register?");
        }
    }

    /// Removes a component from an entity.
    pub fn detach_component<C: 'static>(&self, entity: Entity) -> Option<C> {
        let mut set = self.components.get_mut::<C>()?;
        set.remove(entity.into())
    }

    /// Retrieves a component from an entity. Can be mutable or immutable.
    pub fn fetch_component<'a, R: ComponentRef<'a>>(&'a self, entity: Entity) -> Option<R::Output> {
        R::fetch(self, entity)
    }

    /// Queries the world for components matching the query type.
    pub fn fetch_components<Q: Query<C>, C>(&self, f: Q) {
        Q::fetch(self, f);
    }

    // -----------------------------------------------------------------------
    // Resource management

    /// Adds a resource to the world.
    pub fn register_resource<R: 'static>(&mut self, resource: R) {
        self.resources.create(resource);
    }

    /// Removes a resource from the world.
    pub fn unregister_resource<R: 'static>(&mut self) -> Option<R> {
        self.resources.destroy::<R>()
    }

    /// Retrieves a resource from the world. Can be mutable or immutable.
    pub fn fetch_resource<'a, R: ResourceRef<'a>>(&'a self) -> Option<R::Output> {
        R::fetch(self)
    }

    // -----------------------------------------------------------------------
    // Apply commands.

    /// Applies a command to the world.
    pub fn apply(&mut self, commands: Vec<Command>) {
        for command in commands {
            match command {
                Command::Detach(entity, type_id) => {
                    if let Some(&idx) = self.components.lookup.get(&type_id) {
                        // SAFETY: idx is the right slot
                        let set = &self.components.sets[idx];
                        set.borrow_mut().remove(entity);
                    }
                }
                Command::Kill(entity) => {
                    self.kill_entity(entity);
                }
            }
        }
    }
}

use std::time::{Duration, Instant};

use super::ecs::Entity;

/// Tracks the Owner Entity Id of another entity.
pub(crate) struct Owner(pub Entity);

pub(crate) struct Spawner {
    max_entities: u8,
    pub entities: Vec<Entity>,
    spawn_radius: f32,
    spawn_rate: Duration,
    last_spawn: Instant,
}

impl Spawner {
    pub fn new(max_entities: u8, spawn_radius: f32, spawn_rate: f32) -> Self {
        Self {
            max_entities,
            entities: Vec::new(),
            spawn_radius,
            spawn_rate: Duration::from_secs_f32(spawn_rate),
            last_spawn: Instant::now(),
        }
    }

    pub fn at_capacity(&self) -> bool {
        self.entities.len() >= usize::from(self.max_entities())
    }

    pub fn is_ready(&self) -> bool {
        self.last_spawn.elapsed() > self.spawn_rate
    }

    pub fn reset(&mut self) {
        self.last_spawn = Instant::now();
    }

    pub fn radius(&self) -> f32 {
        self.spawn_radius
    }

    pub fn max_entities(&self) -> u8 {
        self.max_entities
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.push(entity);
    }
}

use std::any::TypeId;

use crate::server::components::Position;
use crate::server::ecs::{Command, Entity, World};
use crate::server::world_map::WorldMap;
use crate::shared::payload::Movement;
use crate::vec2f::Vec2f;

/// Moves entities in the world based on their movement components.
pub fn movement(world: &mut World, map: &WorldMap, tick_rate: f32) -> Vec<Entity> {
    let mut moved = Vec::new();
    let mut remove = Vec::new();

    world.fetch_components(|entity, pos: &mut Position, movement: &mut Movement| {
        let Movement(ref mut velocity, speed) = *movement;
        if *velocity == Vec2f::ZERO {
            // Remove the movement from the entity.
            remove.push(Command::Detach(entity, TypeId::of::<Movement>()));
            return;
        }

        let original_pos = pos.0;
        let speed_delta = f32::from(speed.clamp(1, 3));
        let travel = speed_delta * tick_rate;

        if travel >= velocity.length() {
            // Step distance is smaller than travel requirements.
            pos.0 += *velocity;
            *velocity = Vec2f::ZERO;
            remove.push(Command::Detach(entity, TypeId::of::<Movement>()));
        } else {
            // Move the position using the velocity.
            let direction = *velocity;
            let disp = direction.scale(travel);
            pos.0 += disp;
            *velocity -= disp;
        }

        // Ensure the position remains within the map.
        pos.0 = map.clamp_bounds(pos.0);

        // Mark the entity as moved.
        if original_pos != pos.0 {
            moved.push(entity);
        }
    });

    // Remove entities that have no movement left.
    world.apply(remove);

    moved
}

use std::collections::HashSet;

use crate::server::components::Position;
use crate::server::ecs::{Entity, World};
use crate::server::world_map::WorldMap;
use crate::shared::payload::Movement;
use crate::vec2f::Vec2f;

/// Moves entities in the world based on their movement components.
pub fn movement(world: &mut World, map: &WorldMap, tick_rate: f32) -> HashSet<Entity> {
    let mut moved = HashSet::new();

    world.fetch_components(|entity, pos: &mut Position, movement: &mut Movement| {
        let Movement(ref mut velocity, speed) = *movement;
        if *velocity == Vec2f::ZERO {
            return;
        }

        let original_pos = pos.0;
        let speed_delta = f32::from(speed.clamp(1, 3));
        let travel = speed_delta * tick_rate;

        if travel >= velocity.length() {
            // Step distance is smaller than travel requirements.
            pos.0 += *velocity;
            *velocity = Vec2f::ZERO;
        } else {
            // Move the position using the velocity.
            let direction = velocity.normalized();
            let disp = direction.scale(travel);
            pos.0 += disp;
            *velocity -= disp;
        }

        // Ensure the position remains within the map.
        pos.0 = map.clamp_bounds(pos.0);

        // Mark the entity as moved.
        if original_pos == pos.0 {
            *velocity = Vec2f::ZERO;
        } else {
            moved.insert(entity);
        }
    });

    moved
}

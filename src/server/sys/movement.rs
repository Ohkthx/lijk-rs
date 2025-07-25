use std::collections::HashSet;

use crate::server::ecs::{Entity, World};
use crate::server::world_map::WorldMap;
use crate::shared::node::Node2d;
use crate::shared::payload::Movement;
use crate::shared::shape::Rectangle;
use crate::shared::transform::Transform;
use crate::utils::SpatialHash;
use crate::vec2f::Vec2f;

/// Moves entities in the world based on their movement components.
pub fn movement(
    world: &mut World,
    map: &WorldMap,
    gps: &mut SpatialHash,
    tick_rate: f32,
) -> HashSet<Entity> {
    let mut moved = HashSet::new();

    world.fetch_components(
        |entity: Entity,
         geometry: &Rectangle,
         transform: &mut Transform,
         movement: &mut Movement| {
            let Movement(ref mut velocity, speed) = *movement;

            if *velocity == Vec2f::ZERO {
                return; // No movement required.
            }

            let old_pos = transform.position;
            let mut new_pos = transform.position;
            let speed_delta = f32::from(speed.clamp(1, 3));
            let travel = speed_delta * tick_rate;

            if travel >= velocity.length() {
                // Step distance is smaller than travel requirements.
                new_pos += *velocity;
                *velocity = Vec2f::ZERO;
            } else {
                // Move the position using the velocity.
                let direction = velocity.normalized();
                let disp = direction.scale(travel);
                new_pos += disp;
                *velocity -= disp;
            }

            // Ensure the position remains within the map.
            new_pos = map.clamp_bounds(new_pos);
            let node = Node2d::from((*geometry, Transform::with_position(new_pos)));

            // Check nearby entities at the new position.
            let entities = gps.query(new_pos, 2.0);
            for (other, other_pos) in entities.iter().map(|(e, p)| (Entity::from(*e), *p)) {
                if other == entity {
                    continue;
                }

                let other_transform = Transform::with_position(*other_pos);
                let other_node = Node2d::from((*geometry, other_transform));
                if node.intersects(&other_node) {
                    // Collision detected.
                    *velocity = Vec2f::ZERO; // Stop movement.
                    new_pos = old_pos; // Revert to old position.
                    break;
                }
            }

            // Mark the entity as moved.
            if old_pos == new_pos {
                *velocity = Vec2f::ZERO;
            } else {
                transform.position = new_pos;
                gps.insert(transform.position, entity.into());
                moved.insert(entity);
            }
        },
    );

    moved
}

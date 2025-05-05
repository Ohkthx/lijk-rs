use rand::random_range;

use crate::server::ai::{AiState, BasicAi};
use crate::server::components::Position;
use crate::server::core::LastTarget;
use crate::server::ecs::World;
use crate::shared::payload::Movement;
use crate::vec2f::Vec2f;

pub fn ai(world: &mut World) {
    world.fetch_components(
        |_entity,
         pos: &Position,
         movement: &mut Movement,
         LastTarget(target): &LastTarget,
         ai: &mut BasicAi| {
            let target_pos = if let Some(target) = target {
                world.fetch_component::<&Position>(*target)
            } else {
                None
            };

            match ai.state {
                AiState::Pursue => {
                    let Some(entity_pos) = target_pos else {
                        ai.set_state(AiState::Wander(3.0, 1));
                        return;
                    };

                    if (pos.0 - entity_pos.0).length() > 5.0 {
                        // Out of reach, start wandering.
                        ai.set_state(AiState::Wander(3.0, 1));
                    } else {
                        // Update the movement vector to follow the entity.
                        movement.0 = entity_pos.0 - pos.0;
                    }
                }
                AiState::Wander(radius, speed) => {
                    if let Some(entity_pos) = target_pos {
                        if (pos.0 - entity_pos.0).length() <= 5.0 {
                            // Within range, begin pursuing.
                            ai.set_state(AiState::Pursue);
                            return;
                        }
                    }

                    if movement.0 == Vec2f::ZERO {
                        let vec_x = random_range(-radius..=radius);
                        let vec_y = random_range(-radius..=radius);
                        *movement = Movement(Vec2f(vec_x, vec_y), speed);
                    }
                }
                AiState::Idle => movement.0 = Vec2f::ZERO,
            }
        },
    );
}

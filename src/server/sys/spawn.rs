use std::collections::{HashMap, HashSet};

use rand::random_range;

use crate::server::components::Position;
use crate::server::core::Slime;
use crate::server::ecs::{Entity, World};
use crate::server::spawner::{Owner, Spawner};
use crate::server::world_map::WorldMap;
use crate::vec2f::Vec2f;

pub fn spawn(world: &mut World, map: &WorldMap) -> HashSet<Entity> {
    let mut to_spawn: HashMap<Entity, Vec<Position>> = HashMap::new();
    let mut spawned = HashSet::new();

    world.fetch_components(|entity, pos: &Position, spawner: &mut Spawner| {
        if spawner.at_capacity() || !spawner.is_ready() {
            return;
        }

        // Obtain the location of the spawned entity.
        let offset = random_range(-spawner.radius()..=spawner.radius());
        let dest = Vec2f(pos.0.0 + offset, pos.0.1 + offset);
        let entity_pos = Position(map.clamp_bounds(dest));
        to_spawn.entry(entity).or_default().push(entity_pos);

        spawner.reset();
    });

    // Spawn the entity.
    for (spawner_id, positions) in to_spawn {
        for pos in positions {
            let entity_id = Slime::spawn(world, pos.0);
            world.attach_component(entity_id, Owner(spawner_id));
            if let Some(mut spawner) = world.fetch_component::<&mut Spawner>(spawner_id) {
                spawner.add_entity(entity_id);
                spawned.insert(entity_id);
            } else {
                world.kill_entity(entity_id);
            }
        }
    }

    spawned
}

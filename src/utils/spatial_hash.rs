#![allow(dead_code)]
use std::collections::HashMap;

use crate::vec2f::Vec2f;

type Entity = u32;

/// `Cell` is a 2D cell used for spatial hashing.
#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug)]
struct Cell(i32, i32);

/// `SpatialHash` is a spatial hash used for tracking entities in a 2D space.
#[derive(Debug)]
pub struct SpatialHash {
    cell_size: f32,                             // Size of each cell.
    inverse_cell_size: f32,                     // Inverse of the cell size.
    cells: HashMap<Cell, Vec<(Entity, Vec2f)>>, // Maps Cell => Vec of entities and their true positions.
    lookup: HashMap<u32, (Cell, usize)>,        // Maps entity to its cell and index in the cells.
}

impl SpatialHash {
    /// Creates a new `SpatialHash` used for tracking entities.
    pub fn new(cell_size: f32) -> Self {
        let inverse_cell_size = 1.0 / cell_size;
        Self {
            cell_size,
            inverse_cell_size,
            cells: HashMap::new(),
            lookup: HashMap::new(),
        }
    }

    /// Converts from Cell to Vec2f which is used out of spatial hash context.
    #[allow(clippy::cast_precision_loss)]
    fn convert_cell(&self, cell: Cell) -> Vec2f {
        Vec2f(
            (cell.0 as f32) * self.cell_size,
            (cell.1 as f32) * self.cell_size,
        )
    }

    /// Converts from Vec2f to Cell which is used in spatial hash context.
    #[allow(clippy::cast_possible_truncation)]
    fn convert_vec2f(&self, pos: Vec2f) -> Cell {
        Cell(
            (pos.0 * self.inverse_cell_size).floor() as i32,
            (pos.1 * self.inverse_cell_size).floor() as i32,
        )
    }

    /// Inserts an entity, removing from the old position.
    pub fn insert(&mut self, pos: Vec2f, entity: Entity) {
        let new_cell = self.convert_vec2f(pos);

        if let Some(&(old_cell, old_idx)) = self.lookup.get(&entity) {
            if old_cell == new_cell {
                // No change in cell, just update position.
                if let Some(bucket) = self.cells.get_mut(&new_cell) {
                    bucket[old_idx].1 = pos;
                    return;
                }
            }

            if let Some(bucket) = self.cells.get_mut(&old_cell) {
                let last_idx = bucket.len() - 1;
                bucket.swap_remove(old_idx);

                if old_idx != last_idx {
                    // Update the moved‐entity’s index in lookup.
                    let (moved_entity, _) = bucket[old_idx];
                    self.lookup.get_mut(&moved_entity).unwrap().1 = old_idx;
                }

                // Flush the old cell if it’s empty.
                if bucket.is_empty() {
                    self.cells.remove(&old_cell);
                }
            }
        }

        // Push the entity into a bucket.
        let bucket = self.cells.entry(new_cell).or_default();
        bucket.push((entity, pos));
        let idx = bucket.len() - 1;

        // Update lookup table.
        self.lookup.insert(entity, (new_cell, idx));
    }

    /// Remove the entity from the spatial hash.
    pub fn remove(&mut self, entity: u32) {
        if let Some((cell, idx)) = self.lookup.remove(&entity) {
            if let Some(bucket) = self.cells.get_mut(&cell) {
                let last_idx = bucket.len() - 1;
                bucket.swap_remove(idx);

                if idx != last_idx {
                    // Update the moved‐entity’s index in lookup.
                    let (moved_ent, _) = bucket[idx];
                    self.lookup.get_mut(&moved_ent).unwrap().1 = idx;
                }

                // Flush the old cell if it’s empty.
                if bucket.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    /// Obtains all entities within the given radius of the position.
    #[allow(clippy::cast_possible_truncation)]
    pub fn query(&self, pos: Vec2f, radius: f32) -> Vec<(u32, &Vec2f)> {
        let origin = self.convert_vec2f(pos);
        let cell_radius = (radius * self.inverse_cell_size).ceil() as i32;
        let r2 = radius * radius;

        // Collections of entities that are within the radius.
        let mut hits = Vec::new();

        // Check the surrounding cells from the point of origin.
        // Note: This is due to a position being really close to a boundary.
        for dx in -cell_radius..=cell_radius {
            for dy in -cell_radius..=cell_radius {
                let cell = Cell(origin.0 + dx, origin.1 + dy);

                if let Some(bucket) = self.cells.get(&cell) {
                    for (entity, entity_pos) in bucket {
                        if entity_pos.distance_squared(pos) <= r2 {
                            hits.push((*entity, entity_pos)); // Entity is within the radius.
                        }
                    }
                }
            }
        }

        hits
    }
}

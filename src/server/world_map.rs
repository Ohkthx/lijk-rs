use crate::{shared::box_2d::Box2D, vec2f::Vec2f};

/// Simple implementation of the game world map.
pub(crate) struct WorldMap {
    bounds: Box2D,
}

impl WorldMap {
    /// Creates a new `WorldMap` instance with the specified center, length, and width.
    pub fn new(center: Vec2f, x_width: f32, y_length: f32) -> Self {
        let mut bounds = Box2D::new(Vec2f::ZERO, x_width, y_length);
        bounds.center_on(center);

        Self { bounds }
    }

    /// Gets the spawn point for new entities in the world.
    pub fn spawn_point(&self) -> &Vec2f {
        self.bounds.center()
    }

    /// Checks if the given position is within the bounds of the world map.
    pub fn in_bounds(&self, pos: Vec2f) -> bool {
        self.bounds.contains(pos)
    }

    /// Clamps the position to be withing the map bounds.
    pub fn clamp_bounds(&self, pos: Vec2f) -> Vec2f {
        self.bounds.clamp(pos)
    }
}

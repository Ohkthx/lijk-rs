use crate::vec2f::Vec2f;

/// A transform in a 2D space.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Transform {
    pub position: Vec2f, // Top-left position.
    pub origin: Vec2f,   // Pivot point for rotations.
    pub scale: Vec2f,    // Scale of the transformation.
    pub rotation: f32,   // Rotation in degrees.
}

impl Transform {
    /// Creates the identity transform.
    pub fn identity() -> Self {
        Self {
            position: Vec2f::ZERO,
            origin: Vec2f::ZERO,
            scale: Vec2f::ONE,
            rotation: 0.0,
        }
    }

    /// Creates a new transform with the specified position.
    pub fn with_position(position: Vec2f) -> Self {
        Self {
            position,
            ..Self::identity()
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

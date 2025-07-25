use crate::vec2f::Vec2f;

use super::shape::Rectangle;
use super::transform::Transform;

/// Represents a node in a 2D space with geometry and transformation.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Node2d {
    pub geometry: Rectangle,  // Geometry of the node.
    pub transform: Transform, // Transformation applied to the node.
}

impl Node2d {
    /// Detects if the node intersects with another node.
    pub fn intersects(&self, other: &Self) -> bool {
        // Compute AABB for self.
        let a_min = self.transform.position; // top-left
        let a_max = a_min
            + Vec2f(
                self.geometry.width * self.transform.scale.0,
                self.geometry.height * self.transform.scale.1,
            ); // bottom-right

        // Compute AABB for other.
        let b_min = other.transform.position;
        let b_max = b_min
            + Vec2f(
                other.geometry.width * other.transform.scale.0,
                other.geometry.height * other.transform.scale.1,
            );

        // If one is strictly to the left of the other, no overlap
        if a_min.0 > b_max.0 || b_min.0 > a_max.0 {
            return false;
        }

        // On the Y axis, allow equality (touching counts as overlap).
        a_min.1 <= b_max.1 && b_min.1 <= a_max.1
    }
}

impl From<(Rectangle, Transform)> for Node2d {
    fn from((geometry, transform): (Rectangle, Transform)) -> Self {
        Self {
            geometry,
            transform,
        }
    }
}

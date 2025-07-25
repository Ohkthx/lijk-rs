/// Geometry for a 2D rectangle (width × height).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    /// Creates a new rectangle with the specified width and height.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

impl Eq for Rectangle {}

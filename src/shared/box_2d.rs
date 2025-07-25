use crate::vec2f::Vec2f;

/// A 2D Box used to position objects.
pub struct Box2D {
    pub position: Vec2f, // Top-left position.
    pub center: Vec2f,   // Center position.
    pub width: f32,      // Width (x-axis).
    pub length: f32,     // Length (y-axis).
}

impl Box2D {
    /// Creates a new 2D Box.
    pub fn new(position: Vec2f, x_width: f32, y_length: f32) -> Self {
        Self {
            position,
            center: Vec2f(position.0 + x_width / 2.0, position.1 + y_length / 2.0),
            width: x_width,
            length: y_length,
        }
    }

    #[inline]
    fn max_x(&self) -> f32 {
        self.position.0 + self.width
    }

    #[inline]
    fn max_y(&self) -> f32 {
        self.position.1 + self.length
    }

    /// Returns the center point for the box.
    pub fn center(&self) -> &Vec2f {
        &self.center
    }

    /// Centers the box on the specified position.
    pub fn center_on(&mut self, position: Vec2f) {
        self.center = position;
        self.position = Vec2f(
            position.0 - self.width / 2.0,
            position.1 - self.length / 2.0,
        );
    }

    /// Checks if a given point is within the bounds of the box.
    pub fn contains(&self, point: Vec2f) -> bool {
        let within_x = point.0 >= self.position.0 && point.0 <= self.max_x();
        let within_y = point.1 >= self.position.1 && point.1 <= self.max_y();
        within_x && within_y
    }

    /// Restricts a position to within the bounds of the box.
    /// Ensures the returned point is always within the box, even if the input point is outside.
    pub fn clamp(&self, point: Vec2f) -> Vec2f {
        let x = point.0.clamp(self.position.0, self.max_x());
        let y = point.1.clamp(self.position.1, self.max_y());

        Vec2f(x, y)
    }
}

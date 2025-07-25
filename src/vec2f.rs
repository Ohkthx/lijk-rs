use std::hash::{Hash, Hasher};

use crate::net::traits::{NetDecoder, NetEncoder};
use netcode_derive::{NetDecode, NetEncode};

/// A 2D vector where the components are 32-bit floats.
#[derive(Debug, NetDecode, NetEncode, PartialEq, Clone, Copy)]
pub struct Vec2f(pub f32, pub f32);
impl Vec2f {
    pub const ZERO: Vec2f = Vec2f(0.0, 0.0);
    pub const ONE: Vec2f = Vec2f(1.0, 1.0);

    /// Obtains the length of the vector.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Obtains the squared length of the vector.
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.0 * self.0 + self.1 * self.1
    }

    /// Calculates the dot product of two vectors.
    pub fn dot(self, other: Vec2f) -> f32 {
        self.0 * other.0 + self.1 * other.1
    }

    /// Caculates the distance between two vectors.
    pub fn distance(self, other: Vec2f) -> f32 {
        self.distance_squared(other).sqrt()
    }

    /// Calculates the squared distance between two vectors.
    #[inline]
    pub fn distance_squared(self, other: Vec2f) -> f32 {
        (self.0 - other.0).powi(2) + (self.1 - other.1).powi(2)
    }

    /// Linearly interpolates between two vectors.
    pub fn lerp(self, other: Vec2f, t: f32) -> Vec2f {
        Vec2f(
            self.0 + (other.0 - self.0) * t,
            self.1 + (other.1 - self.1) * t,
        )
    }

    /// Scales the vector by a scalar.
    pub fn scale(self, s: f32) -> Vec2f {
        Vec2f(self.0 * s, self.1 * s)
    }

    /// Normalizes the vector to a unit vector.
    pub fn normalized(self) -> Vec2f {
        let len = self.length();
        if len == 0.0 {
            Vec2f(0.0, 0.0)
        } else {
            Vec2f(self.0 / len, self.1 / len)
        }
    }

    /// Rounds the components of the vector to the nearest integer.
    pub fn round(self) -> Vec2f {
        Vec2f(self.0.round(), self.1.round())
    }
}

impl std::ops::Add for Vec2f {
    type Output = Vec2f;

    fn add(self, other: Vec2f) -> Vec2f {
        Vec2f(self.0 + other.0, self.1 + other.1)
    }
}

impl std::ops::AddAssign for Vec2f {
    fn add_assign(&mut self, other: Vec2f) {
        self.0 += other.0;
        self.1 += other.1;
    }
}

impl std::ops::Sub for Vec2f {
    type Output = Vec2f;

    fn sub(self, other: Vec2f) -> Vec2f {
        Vec2f(self.0 - other.0, self.1 - other.1)
    }
}

impl std::ops::SubAssign for Vec2f {
    fn sub_assign(&mut self, other: Vec2f) {
        self.0 -= other.0;
        self.1 -= other.1;
    }
}

impl Eq for Vec2f {}

impl Hash for Vec2f {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
        self.1.to_bits().hash(state);
    }
}

impl From<Vec2f> for (f32, f32) {
    fn from(v: Vec2f) -> (f32, f32) {
        (v.0, v.1)
    }
}

impl From<(f32, f32)> for Vec2f {
    fn from(v: (f32, f32)) -> Vec2f {
        Vec2f(v.0, v.1)
    }
}

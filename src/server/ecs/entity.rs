use std::hash::Hash;
use std::ops;

/// Represents a unique entity identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity(u32);

impl From<Entity> for usize {
    fn from(value: Entity) -> Self {
        value.0 as usize
    }
}

impl From<Entity> for u32 {
    fn from(value: Entity) -> Self {
        value.0
    }
}

impl From<usize> for Entity {
    fn from(val: usize) -> Self {
        match u32::try_from(val) {
            Ok(value) => Entity(value),
            Err(err) => panic!("Value {val} exceeds u32::MAX: {err}"),
        }
    }
}

impl From<u32> for Entity {
    fn from(val: u32) -> Self {
        Entity(val)
    }
}

impl std::fmt::Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity({})", self.0)
    }
}

impl Hash for Entity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl std::cmp::PartialOrd for Entity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl std::cmp::Ord for Entity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl ops::Add<u32> for Entity {
    type Output = Self;

    fn add(self, rhs: u32) -> Self::Output {
        Entity(self.0 + rhs)
    }
}

impl ops::Sub<u32> for Entity {
    type Output = Self;

    fn sub(self, rhs: u32) -> Self::Output {
        Entity(self.0 - rhs)
    }
}

impl ops::AddAssign<u32> for Entity {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}

impl ops::SubAssign<u32> for Entity {
    fn sub_assign(&mut self, rhs: u32) {
        self.0 -= rhs;
    }
}

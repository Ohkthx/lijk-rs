use crate::vec2f::Vec2f;

/// Represents the position of an entity in the game world.
pub(crate) struct Position(pub Vec2f);

/// Represents movement for an entity with a delta and speed..
pub(crate) struct Movement(pub Vec2f, pub u8);

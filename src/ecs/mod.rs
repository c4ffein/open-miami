// Custom ECS Engine
// Simple, testable architecture with minimal dependencies

pub mod component;
pub mod entity;
pub mod system;
pub mod world;

pub use component::Component;
pub use entity::Entity;
pub use system::System;
pub use world::{Wall, World};

// Re-export common types
pub use crate::math::Vec2;

use std::any::{Any, TypeId};

/// Marker trait for components: any `'static` type can be a component. The
/// downcasting lives on [`AnyComponent`] (what the world actually boxes).
pub trait Component: 'static {}

// Blanket implementation for all 'static types
impl<T: 'static> Component for T {}

/// Object-safe clonable `Any`: what the world's component storage actually
/// boxes. Every stored component must be `Clone` so the whole [`World`] can
/// be snapshotted (the mid-floor `checkpoint` scenario action clones the
/// world and restores it on death).
///
/// [`World`]: crate::ecs::World
pub trait AnyComponent: Any {
    fn clone_box(&self) -> Box<dyn AnyComponent>;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any + Clone> AnyComponent for T {
    fn clone_box(&self) -> Box<dyn AnyComponent> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

impl Clone for Box<dyn AnyComponent> {
    fn clone(&self) -> Self {
        // Explicit deref: `Box<dyn AnyComponent>` is itself `Any + Clone`,
        // so `self.clone_box()` would resolve to the blanket impl ON THE BOX
        // and recurse forever. Dispatch on the inner trait object instead.
        (**self).clone_box()
    }
}

/// Type-safe wrapper for component type IDs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(pub TypeId);

impl ComponentId {
    pub fn of<T: Component>() -> Self {
        ComponentId(TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, PartialEq)]
    struct Velocity {
        x: f32,
        y: f32,
    }

    #[test]
    fn test_component_id_uniqueness() {
        let pos_id = ComponentId::of::<Position>();
        let vel_id = ComponentId::of::<Velocity>();

        assert_ne!(pos_id, vel_id);
    }

    #[test]
    fn test_component_id_consistency() {
        let id1 = ComponentId::of::<Position>();
        let id2 = ComponentId::of::<Position>();

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_component_as_any() {
        #[derive(Debug, Clone, PartialEq)]
        struct Pos {
            x: f32,
        }
        let mut pos = Pos { x: 10.0 };

        let any_ref = pos.as_any();
        let downcast = any_ref.downcast_ref::<Pos>().unwrap();
        assert_eq!(downcast.x, 10.0);

        let any_mut = pos.as_any_mut();
        let downcast_mut = any_mut.downcast_mut::<Pos>().unwrap();
        downcast_mut.x = 30.0;
        assert_eq!(pos.x, 30.0);
    }

    #[test]
    fn test_boxed_component_clones() {
        #[derive(Debug, Clone, PartialEq)]
        struct Tag(u32);
        let boxed: Box<dyn AnyComponent> = Box::new(Tag(7));
        let cloned = boxed.clone();
        assert_eq!(
            cloned.as_ref().as_any().downcast_ref::<Tag>(),
            Some(&Tag(7))
        );
        let back = cloned.into_any().downcast::<Tag>().unwrap();
        assert_eq!(*back, Tag(7));
    }
}

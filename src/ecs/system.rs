use super::World;

/// System trait for implementing game logic
pub trait System {
    fn run(&mut self, world: &mut World, dt: f32);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Counter {
        value: i32,
    }

    /// Bumps every `Counter` by one per run.
    struct CountSystem;

    impl System for CountSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            let entities: Vec<_> = world.query::<Counter>();
            for entity in entities {
                if let Some(counter) = world.get_component_mut::<Counter>(entity) {
                    counter.value += 1;
                }
            }
        }
    }

    #[test]
    fn test_system_runs() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add_component(entity, Counter { value: 0 });

        let mut system = CountSystem;
        system.run(&mut world, 0.016);

        let counter = world.get_component::<Counter>(entity).unwrap();
        assert_eq!(counter.value, 1);
    }

    #[test]
    fn test_system_multiple_runs() {
        let mut world = World::new();
        let entity = world.spawn();
        world.add_component(entity, Counter { value: 0 });

        let mut system = CountSystem;
        for _ in 0..10 {
            system.run(&mut world, 0.016);
        }

        let counter = world.get_component::<Counter>(entity).unwrap();
        assert_eq!(counter.value, 10);
    }
}

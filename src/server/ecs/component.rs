use std::any::{Any, TypeId, type_name};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use super::entity::Entity;
use super::sset::SparseSet;
use super::world::World;

/// A trait for fetching components from the world that can be either mutable or immutable.
pub(crate) trait ComponentRef<'a> {
    type Output: 'a; // Used to specify the output type of the fetch operation.

    /// Fetch a component from the world for the given entity.
    fn fetch(world: &'a World, entity: Entity) -> Option<Self::Output>;
}

impl<'a, C: 'static> ComponentRef<'a> for &'a C {
    type Output = Ref<'a, C>;

    fn fetch(world: &'a World, entity: Entity) -> Option<Self::Output> {
        let guard = world.components.get::<C>()?;
        Ref::filter_map(guard, |set| set.get(entity.into())).ok()
    }
}

impl<'a, C: 'static> ComponentRef<'a> for &'a mut C {
    type Output = RefMut<'a, C>;

    fn fetch(world: &'a World, entity: Entity) -> Option<Self::Output> {
        let guard = world.components.get_mut::<C>()?;
        RefMut::filter_map(guard, |set| set.get_mut(entity.into())).ok()
    }
}

/// Accesses the underlying sparse set for a component type.
pub(crate) trait SetAccess {
    type Output<'b>; // Output type for the component set.
    type Guard<'c>; // Guard type for accessing the sparse set.

    /// Obtains a readable or mutable reference to the sparse set for the component type.
    fn set(world: &World) -> Option<Self::Guard<'_>>;

    /// Retrieves a component from the sparse set for the given entity.
    fn component<'b>(iter: &'b mut Self::Guard<'_>, entity: Entity) -> Option<Self::Output<'b>>;

    /// Iterates over the components in the sparse set.
    fn iter<'b>(iter: &'b mut Self::Guard<'_>) -> impl Iterator<Item = (Entity, Self::Output<'b>)>;

    /// Returns the length of the sparse set.
    fn length(iter: &'_ Self::Guard<'_>) -> usize;
}

impl<C: 'static> SetAccess for &C {
    type Output<'b> = &'b C;
    type Guard<'c> = Ref<'c, SparseSet<C>>;

    fn set(world: &World) -> Option<Self::Guard<'_>> {
        world.components.get()
    }

    fn component<'b>(iter: &'b mut Self::Guard<'_>, entity: Entity) -> Option<Self::Output<'b>> {
        iter.get(entity.into())
    }

    fn iter<'b>(iter: &'b mut Self::Guard<'_>) -> impl Iterator<Item = (Entity, Self::Output<'b>)> {
        iter.iter().map(|(e, c)| (Entity::from(e), c))
    }

    fn length(iter: &'_ Self::Guard<'_>) -> usize {
        iter.length()
    }
}

impl<C: 'static> SetAccess for &mut C {
    type Output<'b> = &'b mut C;
    type Guard<'c> = RefMut<'c, SparseSet<C>>;

    fn set(world: &World) -> Option<Self::Guard<'_>> {
        world.components.get_mut()
    }

    fn component<'b>(iter: &'b mut Self::Guard<'_>, entity: Entity) -> Option<Self::Output<'b>> {
        iter.get_mut(entity.into())
    }

    fn iter<'b>(iter: &'b mut Self::Guard<'_>) -> impl Iterator<Item = (Entity, Self::Output<'b>)> {
        iter.iter_mut().map(|(e, c)| (Entity::from(e), c))
    }

    fn length(iter: &'_ Self::Guard<'_>) -> usize {
        iter.length()
    }
}

/// A trait for managing sparse sets of components.
pub(crate) trait Set {
    /// Provides a reference to the underlying `Any` type.
    fn as_any(&self) -> &dyn Any;
    /// Provides a mutable reference to the underlying `Any` type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Removes a component from the sparse set for the given entity.
    fn remove(&mut self, entity: Entity);
}

impl<C: 'static> Set for SparseSet<C> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, entity: Entity) {
        self.remove(entity.into());
    }
}

/// Storage for components in the world.
#[derive(Default)]
pub(crate) struct ComponentStorage {
    pub(crate) lookup: HashMap<TypeId, usize>, // Lookup table for component types.
    pub(crate) sets: Vec<Box<RefCell<dyn Set>>>, // Sets of components for entities.
}

impl ComponentStorage {
    /// Creates a new set for the given component type where `C` is the component type.
    pub fn create<C: 'static>(&mut self) {
        let id = TypeId::of::<C>();
        if self.lookup.contains_key(&id) {
            return;
        }

        self.lookup.insert(id, self.sets.len());
        self.sets
            .push(Box::new(RefCell::new(SparseSet::<C>::new())));
    }

    /// Destroys the set for the given component type where `C` is the component type.
    pub fn destroy<C: 'static>(&mut self) {
        if let Some(index) = self.lookup.remove(&TypeId::of::<C>()) {
            self.sets.remove(index);
            for v in self.lookup.values_mut() {
                if *v > index {
                    *v -= 1;
                }
            }
        }
    }

    /// Removes an entity and its components from all sparse sets.
    pub fn remove_entity(&mut self, entity: Entity) {
        for set in &mut self.sets {
            set.borrow_mut().remove(entity);
        }
    }

    /// Obtains a readable reference to the sparse set for the given component type where `C` is the component type.
    pub fn get<C: 'static>(&self) -> Option<Ref<SparseSet<C>>> {
        let idx = *self.lookup.get(&TypeId::of::<C>())?;
        let boxed = self.sets.get(idx)?;

        // Attempt to get a readable reference to the sparse set.
        let Ok(ref_set) = boxed.try_borrow() else {
            panic!(
                "Tried to immutably access component `{}` but it was already being accessed mutably.",
                type_name::<C>()
            )
        };

        // Downcast to the concrete type.
        Some(Ref::map(ref_set, |set| {
            set.as_any().downcast_ref::<SparseSet<C>>().unwrap()
        }))
    }

    /// Obtains a mutable reference to the sparse set for the given component type where `C` is the component type.
    pub fn get_mut<C: 'static>(&self) -> Option<RefMut<SparseSet<C>>> {
        let idx = *self.lookup.get(&TypeId::of::<C>())?;
        let boxed = self.sets.get(idx)?;

        // Attempt to get a mutable reference to the sparse set.
        let Ok(ref_set) = boxed.try_borrow_mut() else {
            panic!(
                "Tried to mutably access component `{}` but it was already being used.",
                type_name::<C>()
            )
        };

        // Downcast to the concrete type.
        Some(RefMut::map(ref_set, |set| {
            set.as_any_mut().downcast_mut::<SparseSet<C>>().unwrap()
        }))
    }
}

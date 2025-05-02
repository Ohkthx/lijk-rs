#![allow(dead_code)]

use std::any::{Any, TypeId, type_name};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use super::world::World;

/// A trait for fetching resources from the world that can be either mutable or immutable.
pub(crate) trait ResourceRef<'a> {
    type Output: 'a; // Used to specify the output type of the fetch operation.

    /// Fetch a resource from the world for the given entity.
    fn fetch(world: &'a World) -> Option<Self::Output>;
}

impl<'a, R: 'static> ResourceRef<'a> for &'a R {
    type Output = Ref<'a, R>;

    fn fetch(world: &'a World) -> Option<Self::Output> {
        world.resources.get::<R>()
    }
}

impl<'a, R: 'static> ResourceRef<'a> for &'a mut R {
    type Output = RefMut<'a, R>;

    fn fetch(world: &'a World) -> Option<Self::Output> {
        world.resources.get_mut::<R>()
    }
}

/// Storage for all resources, similar to the `ComponentStorage` implementation.
#[derive(Default)]
pub(crate) struct ResourceStorage {
    lookup: HashMap<TypeId, usize>, // Lookup table to get index of resource.
    data: Vec<Box<RefCell<dyn Any>>>, // Storage for resources.
}

impl ResourceStorage {
    /// Creates a new resource of the given type where `R` is the resource type.
    pub fn create<R: 'static>(&mut self, resource: R) {
        let id = TypeId::of::<R>();
        if self.lookup.contains_key(&id) {
            return; // Resource already exists, do not overwrite.
        }

        self.lookup.insert(id, self.data.len());
        self.data.push(Box::new(RefCell::new(Some(resource))));
    }

    /// Destroys the resource of the given type where `R` is the resource type.
    pub fn destroy<R: 'static>(&mut self) -> Option<R> {
        let idx = *self.lookup.get(&TypeId::of::<R>())?;
        self.lookup.remove(&TypeId::of::<R>());

        // Remove the element from `self.data`; swap_remove for O(1) removal.
        let boxed = if idx < self.data.len() - 1 {
            self.data.swap_remove(idx)
        } else {
            self.data.pop().unwrap()
        };

        // Attempt to get a mutable reference to the resource.
        let Ok(mut any_ref) = boxed.try_borrow_mut() else {
            panic!(
                "Attempted to destroy {} resource while it was being accessed.",
                type_name::<R>()
            );
        };

        // Downcast to the concrete `Option<R>` and and extract.
        any_ref.downcast_mut::<Option<R>>()?.take()
    }

    /// Obtains a readable reference for the given resource type where `R` is the resource type.
    pub fn get<R: 'static>(&self) -> Option<Ref<R>> {
        let idx = *self.lookup.get(&TypeId::of::<R>())?;
        let boxed = self.data.get(idx)?;

        // Attempt to get a readable reference to the resource.
        let Ok(any_ref) = boxed.try_borrow() else {
            panic!(
                "Tried to immutably access resource `{}` but it was already being accessed mutably.",
                type_name::<R>()
            )
        };

        // Map the borrowed `Any` -> `Option<R>` -> `&R`
        Ref::filter_map(any_ref, |any_val| {
            any_val.downcast_ref::<Option<R>>()?.as_ref()
        })
        .ok()
    }

    /// Obtains a writable reference for the given resource type where `R` is the resource type.
    pub fn get_mut<R: 'static>(&self) -> Option<RefMut<R>> {
        let idx = *self.lookup.get(&TypeId::of::<R>())?;
        let boxed = self.data.get(idx)?;

        let Ok(any_ref) = boxed.try_borrow_mut() else {
            panic!(
                "Tried to mutably access resource `{}` but it was already being used.",
                type_name::<R>()
            )
        };

        // Map the borrowed-mutable `Any` -> `Option<R>` -> `&mut R`
        RefMut::filter_map(any_ref, |any_val| {
            any_val.downcast_mut::<Option<R>>()?.as_mut()
        })
        .ok()
    }
}

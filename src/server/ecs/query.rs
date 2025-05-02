use super::component::SetAccess;
use super::entity::Entity;
use super::world::World;

/// Query trait for fetching components from the world.
pub(crate) trait Query<P>: Sized {
    /// Fetch components from the world and apply the provided function.
    fn fetch(world: &World, f: Self);
}

/// Single component query.
impl<T, F> Query<(T,)> for F
where
    T: SetAccess,
    F: FnMut(Entity, T::Output<'_>) + FnMut(Entity, T),
{
    fn fetch(world: &World, mut f: F) {
        if let Some(mut storage) = T::set(world) {
            for (entity, component) in T::iter(&mut storage) {
                f(entity, component);
            }
        }
    }
}

/// Two component query.
impl<T, U, F> Query<(T, U)> for F
where
    T: SetAccess,
    U: SetAccess,
    F: FnMut(Entity, T::Output<'_>, U::Output<'_>) + FnMut(Entity, T, U),
{
    fn fetch(world: &World, mut f: F) {
        let (Some(mut set_t), Some(mut set_u)) = (T::set(world), U::set(world)) else {
            return;
        };

        // Iterate over the smaller set to optimize performance.
        let len_t = T::length(&set_t);
        let len_u = U::length(&set_u);

        if len_t <= len_u {
            for (entity, comp_t) in T::iter(&mut set_t) {
                if let Some(comp_u) = U::component(&mut set_u, entity) {
                    f(entity, comp_t, comp_u);
                }
            }
        } else {
            for (entity, comp_u) in U::iter(&mut set_u) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    f(entity, comp_t, comp_u);
                }
            }
        }
    }
}

/// Three component query.
impl<T, U, V, F> Query<(T, U, V)> for F
where
    T: SetAccess,
    U: SetAccess,
    V: SetAccess,
    F: FnMut(Entity, T::Output<'_>, U::Output<'_>, V::Output<'_>) + FnMut(Entity, T, U, V),
{
    fn fetch(world: &World, mut f: F) {
        let (Some(mut set_t), Some(mut set_u), Some(mut set_v)) =
            (T::set(world), U::set(world), V::set(world))
        else {
            return;
        };

        // Iterate over the smaller set to optimize performance.
        let len_t = T::length(&set_t);
        let len_u = U::length(&set_u);
        let len_v = V::length(&set_v);

        if len_t <= len_u && len_t <= len_v {
            for (entity, comp_t) in T::iter(&mut set_t) {
                if let Some(comp_u) = U::component(&mut set_u, entity) {
                    if let Some(comp_v) = V::component(&mut set_v, entity) {
                        f(entity, comp_t, comp_u, comp_v);
                    }
                }
            }
        } else if len_u <= len_t && len_u <= len_v {
            for (entity, comp_u) in U::iter(&mut set_u) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    if let Some(comp_v) = V::component(&mut set_v, entity) {
                        f(entity, comp_t, comp_u, comp_v);
                    }
                }
            }
        } else {
            for (entity, comp_v) in V::iter(&mut set_v) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    if let Some(comp_u) = U::component(&mut set_u, entity) {
                        f(entity, comp_t, comp_u, comp_v);
                    }
                }
            }
        }
    }
}

/// Four component query.
impl<T, U, V, W, F> Query<(T, U, V, W)> for F
where
    T: SetAccess,
    U: SetAccess,
    V: SetAccess,
    W: SetAccess,
    F: FnMut(Entity, T::Output<'_>, U::Output<'_>, V::Output<'_>, W::Output<'_>)
        + FnMut(Entity, T, U, V, W),
{
    fn fetch(world: &World, mut f: F) {
        let (Some(mut set_t), Some(mut set_u), Some(mut set_v), Some(mut set_w)) =
            (T::set(world), U::set(world), V::set(world), W::set(world))
        else {
            return;
        };

        // Iterate over the smaller set to optimize performance.
        let len_t = T::length(&set_t);
        let len_u = U::length(&set_u);
        let len_v = V::length(&set_v);
        let len_w = W::length(&set_w);

        if len_t <= len_u && len_t <= len_v && len_t <= len_w {
            for (entity, comp_t) in T::iter(&mut set_t) {
                if let Some(comp_u) = U::component(&mut set_u, entity) {
                    if let Some(comp_v) = V::component(&mut set_v, entity) {
                        if let Some(comp_w) = W::component(&mut set_w, entity) {
                            f(entity, comp_t, comp_u, comp_v, comp_w);
                        }
                    }
                }
            }
        } else if len_u <= len_t && len_u <= len_v && len_u <= len_w {
            for (entity, comp_u) in U::iter(&mut set_u) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    if let Some(comp_v) = V::component(&mut set_v, entity) {
                        if let Some(comp_w) = W::component(&mut set_w, entity) {
                            f(entity, comp_t, comp_u, comp_v, comp_w);
                        }
                    }
                }
            }
        } else if len_v <= len_t && len_v <= len_u && len_v <= len_w {
            for (entity, comp_v) in V::iter(&mut set_v) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    if let Some(comp_u) = U::component(&mut set_u, entity) {
                        if let Some(comp_w) = W::component(&mut set_w, entity) {
                            f(entity, comp_t, comp_u, comp_v, comp_w);
                        }
                    }
                }
            }
        } else {
            for (entity, comp_w) in W::iter(&mut set_w) {
                if let Some(comp_t) = T::component(&mut set_t, entity) {
                    if let Some(comp_u) = U::component(&mut set_u, entity) {
                        if let Some(comp_v) = V::component(&mut set_v, entity) {
                            f(entity, comp_t, comp_u, comp_v, comp_w);
                        }
                    }
                }
            }
        }
    }
}


use std::sync::Arc;
use std::ops::{Deref, DerefMut};
use std::collections::BTreeMap;
use std::any::Any;

use strata_traits::Resource;

use crate::systems::SystemParam;
use crate::engine::Engine;
use crate::scheduler::Accessor;
use crate::archetypes::ComponentId;
use crate::scheduler::Unsafe;
use crate::archetypes::Archetypes;
use crate::scheduler::UnsafeRef;

pub type ResourceId = u64;

/// Stores resources
pub struct Resources {
    resources: BTreeMap<ResourceId, Box<dyn Any + Send + Sync>>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
        }
    }

    pub fn insert<R: Resource + std::marker::Send + std::marker::Sync>(&mut self, res: R) {
        self.resources.insert(R::__internal_id(), Box::new(Unsafe::new(res)));
    }

    pub unsafe fn get<R: Resource>(&self) -> &'static mut R {
        if let Some(res) = self.resources.get(&R::__internal_id()) {
            if let Some(res) = res.downcast_ref::<Unsafe<R>>() {
                &mut *res.get()
            } else {
                panic!("Resource contained null pointer (internal error)")
            }
        } else {
            panic!("Attempted to get a resource that has not been loaded!")
        }
    }
}

pub struct Res<R: Resource>(&'static R);
pub struct ResMut<R: Resource>(&'static mut R);

impl<R: Resource + 'static> SystemParam for Res<R> {
    fn fetch_param(engine: UnsafeRef<Engine>) -> Self  {
        Res(unsafe { engine.get().resources.get::<R>() })
    }

    fn fetch_access() -> Vec<Accessor> {
        vec![Accessor::Res(R::__internal_id())]
    }

    fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>) {
        // do nothing
    }
}

impl<R: Resource> Deref for Res<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<R: Resource> SystemParam for ResMut<R> {
    fn fetch_param(engine: UnsafeRef<Engine>) -> Self {
        ResMut(unsafe { engine.get().resources.get::<R>() })
    }

    fn fetch_access() -> Vec<Accessor> {
        vec![Accessor::ResMut(R::__internal_id())]
    }

    fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>) {
        // do nothing
    }
}

impl<R: Resource> Deref for ResMut<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<R: Resource> DerefMut for ResMut<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}
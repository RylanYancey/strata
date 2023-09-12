
use std::sync::Arc;

use rayon::prelude::*;

use crate::engine::Engine;
use crate::systems::System;
use crate::resources::ResourceId;
use crate::commands::Commands;
use crate::archetypes::ComponentId;
use crate::resources::Resources;
use crate::archetypes::Archetypes;

pub type SystemIndex = usize;

pub struct Scheduler {
    systems: Vec<Node>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn insert(&mut self, system: Box<dyn System + Send + Sync>) {
        let mut new = Node {
            access: system.accessors(),
            system: Arc::new(system),
            edges: vec![self.systems.len()],
            has_ran: Unsafe::new(false),
        };

        let index = self.systems.len();
        for (i, node) in self.systems.iter_mut().enumerate() {
            if !node.conflicts_with(&new) {
                new.edges.push(i);
                node.edges.push(index);
            }
        }

        self.systems.push(new);
    }

    pub fn execute(&mut self, engine: UnsafeRef<Engine>) {
        // for each system
        for i in 0..self.systems.len() {
            // if the system has not been ran
            if !self.systems[i].has_ran() {
                // run each system at each edge (with which it has no conflicts)
                rayon::scope(|s| {
                    // for each edge
                    for edge in self.systems[i].edges.iter() {
                        // if the system has not been ran,
                        if !self.systems[*edge].has_ran() {
                            // run it.
                            let sys = self.systems[*edge].system.clone();
                            let eng = engine.clone();
                            s.spawn(move |_| sys.execute(eng));
                            self.systems[*edge].set_has_ran(true);
                        }
                    }
                });
            }
        }

        for node in self.systems.iter_mut() {
            node.set_has_ran(false);
        }
    }

    pub fn get_queries(&self, queries: &mut Vec<Vec<ComponentId>>) {
        for node in self.systems.iter() {
            node.system.queries(queries)
        }   
    } 
}

struct Node {
    system: Arc<Box<dyn System + Send + Sync>>,
    access: Vec<Accessor>,
    edges: Vec<SystemIndex>,
    has_ran: Unsafe<bool>,
}

impl Node {
    pub fn conflicts_with(&self, other: &Node) -> bool {
        for accessor in self.access.iter() {
            match *accessor {
                Accessor::Ref(id) => {
                    if other.has(Accessor::Mut(id)) {
                        return true
                    }
                },
                Accessor::Mut(id) => {
                    if other.has(Accessor::Mut(id)) || other.has(Accessor::Ref(id)) {
                        return true
                    }
                },
                Accessor::Res(id) => {
                    if other.has(Accessor::ResMut(id)) {
                        return true
                    }
                },
                Accessor::ResMut(id) => {
                    if other.has(Accessor::ResMut(id)) || other.has(Accessor::Res(id)) {
                        return true
                    }
                },
                Accessor::None => { /* do nothing */ }
            }
        }

        false
    }

    pub fn set_has_ran(&self, v: bool) {
        unsafe { *(self.has_ran.get()) = v; }
    }

    pub fn has_ran(&self) -> bool {
        unsafe { *(self.has_ran.get()) }
    }

    pub fn has(&self, accessor: Accessor) -> bool {
        self.access.contains(&accessor)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Accessor {
    None,
    Ref(ComponentId),
    Mut(ComponentId),
    Res(ResourceId),
    ResMut(ResourceId),
}

#[repr(transparent)]
pub struct Unsafe<T: ?Sized> {
    value: T,
}

unsafe impl<T: ?Sized + Sync> Sync for Unsafe<T> {}
unsafe impl<T: ?Sized + Send> Send for Unsafe<T> {}

impl<T> Unsafe<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value
        }
    }
}

impl<T: ?Sized> Unsafe<T> {
    pub const fn get(&self) -> *mut T {
        &self.value as *const T as *mut T
    }
}

#[repr(transparent)]
pub struct UnsafeRef<T: ?Sized> {
    ptr: *mut T,
}

unsafe impl<T: ?Sized + Sync> Sync for UnsafeRef<T> {}
unsafe impl<T: ?Sized + Send> Send for UnsafeRef<T> {}

impl<T> UnsafeRef<T> {
    pub const fn new(value: &T) -> Self {
        Self {
            ptr: value as *const T as *mut T
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*(self.ptr) }
    }

    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *(self.ptr) }
    }
}

impl<T: ?Sized> Clone for UnsafeRef<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr.clone() }
    }
}
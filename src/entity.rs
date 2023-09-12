
use crate::archetypes::Archetype;
use crate::archetypes::ComponentId;
use crate::anon::Anon;
use crate::archetypes::{Column, TableIndex};

use strata_traits::Component;

pub struct Entity {
    pub(crate) components: Vec<Anon>,
    pub(crate) archetype: Archetype
}

impl Entity {
    pub(crate) fn new() -> Self {
        Self {
            components: Vec::new(),
            archetype: Archetype::new()
        }
    }

    pub(crate) fn hash(&mut self) {
        for anon in self.components.iter() {
            self.archetype.add(anon.id())
        }
    }

    pub(crate) fn remove(&mut self, id: ComponentId) {
        for i in 0..self.components.len() {
            if id == self.components[i].id() {
                self.components[i].clear();
                self.components.remove(i);
                return;
            }
        }
    }

    pub(crate) fn insert_anon(&mut self, anon: Anon) {
        for i in 0..self.components.len() {
            if anon.id() == self.components[i].id() {
                self.components[i] = anon;
                return;
            }
        }

        self.components.push(anon);
    }

    pub(crate) fn pop(&mut self) -> Option<Anon> {
        self.components.pop()
    }

    pub fn insert<C: Component>(&mut self, cmp: C) {
        self.components.push(Anon::new::<C>(cmp));
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct EntityIndex {
    pub table: TableIndex,
    pub col: Column,
}

pub struct EntityIndexIter {
    pub table: TableIndex,
    pub col: usize,
    pub len: usize,
}

impl Iterator for EntityIndexIter {
    type Item = EntityIndex;

    fn next(&mut self) -> Option<Self::Item> {
        if self.col == self.len {
            None
        } else {
            self.col += 1;
            Some(EntityIndex {
                table: self.table,
                col: self.col,
            })
        }
    }
}

pub struct EntityIndexChain {
    pub iters: Vec<EntityIndexIter>,
}

impl EntityIndexChain {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            iters: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, iter: EntityIndexIter) {
        self.iters.push(iter)
    }
}

impl Iterator for EntityIndexChain {
    type Item = EntityIndex;

    fn next(&mut self) -> Option<Self::Item> {
        // get the last iter if it exists
        if let Some(curr) = self.iters.last_mut() {
            let out = curr.next();

            // if iters is empty now, move to the next one. 
            if curr.col == curr.len {
                self.iters.pop();
            }

            out   
        } else {
            // else, this iter is done. 
            None
        }
    }
}
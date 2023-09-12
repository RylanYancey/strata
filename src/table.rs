
use std::sync::{Mutex, MutexGuard};
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::collections::HashSet;

use indexmap::IndexMap;
use strata_traits::Component;

use crate::archetypes::{ComponentId, Column};
use crate::anon::{AnonVec, Anon, AnonIter};
use crate::entity::{Entity, EntityIndexIter};

pub struct Table {
    rows: BTreeMap<ComponentId, AnonVec>,
    queue: Mutex<Queues>,
    update: UnsafeCell<bool>,
    modify: UnsafeCell<bool>,
    num_entities: usize,
}

impl Table {
    pub fn new(mut entity: Entity) -> Self {
        let mut rows = BTreeMap::new();
        while let Some(anon) = entity.pop() {
            if let Some(_) = rows.insert(anon.id(), AnonVec::new(anon)) {
                panic!("Archetypes can only contain one of each component")
            }
        }

        Self {
            rows,
            queue: Mutex::new(Queues::new()),
            update: UnsafeCell::new(false),
            modify: UnsafeCell::new(false),
            num_entities: 1,
        }
    }

    pub fn needs_update(&self) -> bool {
        unsafe { *self.update.get() }
    }

    pub fn needs_modify(&self) -> bool {
        unsafe { *self.modify.get() }
    }

    pub fn is_empty(&self) -> bool {
        self.num_entities == 0
    }

    pub fn contains(&self, ids: &Vec<ComponentId>) -> bool {
        for id in ids.iter() {
            if !self.rows.contains_key(id) { return false }
        }
        true
    }

    pub fn collect<C: Component>(&self) -> Option<AnonIter<C>> {
        if self.is_empty() { return None }

        if let Some(row) = self.rows.get(&C::__internal_id()) {
            return Some(row.iter_as::<C>())
        } else {
            panic!("Attempted to collect component from archetype in which it does not exist")
        }
    }

    pub fn collect_indices(&self, table: usize) -> Option<EntityIndexIter> {
        if !self.is_empty() {
            Some(EntityIndexIter {
                table,
                col: 0,
                len: self.num_entities,
            })
        } else {
            None
        }
    }

    pub fn spawn(&self, mut entity: Entity) {
        let mut queue = self.queue.lock().unwrap();
        queue.spawn.push(entity);
        unsafe { *self.update.get() = true }
    }

    pub fn spawn_group(&self, mut entities: Vec<Entity>) {
        let mut queue = self.queue.lock().unwrap();
        queue.spawn.append(&mut entities);
        unsafe { *self.update.get() = true }
    }

    pub fn destroy_group(&self, mut destroy: Vec<DestroyType>) {
        let mut queue = self.queue.lock().unwrap();
        queue.destroy.append(&mut destroy);
        unsafe { *self.update.get() = true }
    }

    pub fn modify_group(&self, col: Column, mut insert: Vec<Anon>, mut remove: Vec<ComponentId>) {
        let mut queue = self.queue.lock().unwrap();
        if let Some((ins, rem)) = queue.modify.get_mut(&col) {
            if !insert.is_empty() { ins.append(&mut insert) }
            if !remove.is_empty() { rem.append(&mut remove) }
        } else {
            queue.modify.insert(col, (insert, remove));
        }
        unsafe { *self.modify.get() = true; }
    }

    pub fn process_modify(&mut self, entities: &mut Vec<Entity>) {
        let mut queue = self.queue.lock().unwrap();
        queue.destroy.sort();
        queue.destroy.dedup();
        while let Some((col, (mut ins, mut rem))) = queue.modify.pop() {
            if queue.destroy.contains(&DestroyType::Drop(col)) {
                continue;
            }

            let mut entity = self.entity_at(col);
            
            while let Some(rem) = rem.pop() {
                entity.remove(rem);
            }

            while let Some(ins) = ins.pop() {
                entity.insert_anon(ins);
            }

            queue.destroy.push(DestroyType::NoDrop(col));

            entity.hash();
            entities.push(entity);
        }

        unsafe { *self.modify.get() = false; }
    }

    pub fn process_queues(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        
        queue.destroy.sort();
        queue.destroy.dedup();

        // perform all spawns
        while let Some(mut entity) = queue.spawn.pop() {
            self.num_entities += 1;
            while let Some(anon) = entity.pop() {
                if let Some(row) = self.rows.get_mut(&anon.id()) {
                    row.push(anon);
                }
            }
        }

        // perform all destroys
        while let Some(destroy) = queue.destroy.pop() {
            self.num_entities -= 1;
            for (_, row) in self.rows.iter_mut() {
                match destroy {
                    DestroyType::Drop(col) => row.destroy_swap(col),
                    DestroyType::NoDrop(col) => row.destroy_nodrop(col),
                }
            }
        }

        unsafe { *self.update.get() = false; }
    }

    fn entity_at(&self, col: Column) -> Entity {
        let mut entity = Entity::new();
        for (_, row) in self.rows.iter() {
            entity.insert_anon(row.index(col));
        }
        entity
    }
}

struct Queues {
    spawn: Vec<Entity>,
    destroy: Vec<DestroyType>,
    modify: IndexMap<Column, (Vec<Anon>, Vec<ComponentId>)>,
}

impl Queues {
    pub fn new() -> Self {
        Self {
            spawn: Vec::new(),
            destroy: Vec::new(),
            modify: IndexMap::new(),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DestroyType {
    Drop(Column),
    NoDrop(Column),
}

unsafe impl Sync for Table { }
unsafe impl Send for Table { }
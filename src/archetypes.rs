
use std::collections::BTreeMap;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use rayon::prelude::*;
use indexmap::{IndexMap, IndexSet};
use strata_traits::Component;

use crate::anon::AnonIterChain;
use crate::table::Table;
use crate::entity::{Entity, EntityIndexChain};
use crate::commands::Queue;

pub type Column = usize;
pub type TableIndex = usize;
pub type ComponentId = u64;

pub struct Archetypes {
    archetypes: BTreeMap<Archetype, TableIndex>,
    tables: Vec<Table>,
    spawn: Mutex<BTreeMap<Archetype, Vec<Entity>>>,
    cache: HashMap<u64, (Vec<ComponentId>, IndexSet<TableIndex>)>,
    modify: Vec<Entity>,
}

impl Archetypes {
    pub fn new() -> Self {
        Self {
            archetypes: BTreeMap::new(),
            tables: Vec::new(),
            spawn: Mutex::new(BTreeMap::new()),
            modify: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn flush_queues(&mut self) {
        let mut spawn = self.spawn.lock().unwrap();
        
        // Flush everything in "Spawn"
        while let Some((archetype, mut entities)) = spawn.pop_last() {
            // get the first entity
            if let Some(entity) = entities.pop() {
                // check if the archetype already exists
                if let Some(index) = self.archetypes.get(&entity.archetype) {
                    self.tables[*index].spawn_group(entities);
                } else {
                // create the archetype
                    let index = self.tables.len();

                    if let Some(_) = self.archetypes.insert(archetype, index) {
                        panic!("Attempted to create archetype that already exists!")
                    }
                    self.tables.push(Table::new(entity));
    
                    // update the cache with the new archetype index
                    self.cache.par_iter_mut().for_each(|(arch, (ids, indices))| {
                        if self.tables[index].contains(ids) {
                            indices.insert(index);
                        }
                    });
                }
            } else {
                panic!("Queued an empty vector for spawn!")
            }
        }

        // get all the modifies
        for table in self.tables.iter_mut() {
            if table.needs_modify() {
                table.process_modify(&mut self.modify);
            }
        }

        // submit all the modifies
        while let Some(entity) = self.modify.pop() {
            if let Some(index) = self.archetypes.get(&entity.archetype) {
                self.tables[*index].spawn(entity);
            } else {
                let index = self.tables.len();
                if let Some(_) = self.archetypes.insert(entity.archetype, index) {
                    panic!("Attempted to create an archetype that already exists!")
                }
                self.tables.push(Table::new(entity));
                
                // update the cache with the new archetype index
                self.cache.par_iter_mut().for_each(|(arch, (ids, indices))| {
                    if self.tables[index].contains(ids) {
                        indices.insert(index);
                    }
                });
            }
        }

        // process the spawn and destroy queues inside the table.
        for table in self.tables.iter_mut() {
            if table.needs_update() {
                table.process_queues()
            }
        }
    }

    pub fn query(&self, arch: Archetype) -> &IndexSet<TableIndex> {
        if let Some((_, indices)) = self.cache.get(&arch.0) {
            indices
        } else {
            panic!("Attempted to get from a query that does not exist!")
        }
    }

    pub fn add_query(&mut self, ids: Vec<ComponentId>) {
        let mut arch = Archetype::new();
        for id in ids.iter() {
            arch.add(*id);
        }

        self.cache.insert(arch.0, (ids, IndexSet::new()));
    }

    pub fn collect<C: Component>(&self, indices: &IndexSet<TableIndex>) -> AnonIterChain<C> {
        let mut chain = AnonIterChain { iters: Vec::with_capacity(indices.len()) };
        for index in indices.iter() {
            if let Some(iter) = self.tables[*index].collect::<C>() {
                chain.push(iter);
            } 
        }
        chain
    }

    pub fn collect_indices(&self, indices: &IndexSet<TableIndex>) -> EntityIndexChain {
        let mut chain = EntityIndexChain { iters: Vec::with_capacity(indices.len()) };
        for index in indices.iter() {
            if let Some(iter) = self.tables[*index].collect_indices(*index) {
                chain.push(iter);
            }
        }
        chain
    }

    pub fn queue(&self, mut queue: Queue) {
        // queue spawns
        if let Some(ref mut spawn) = queue.spawn {
            let mut selfspawn = self.spawn.lock().unwrap();
            while let Some((archetype, mut entities)) = spawn.pop_first() {
                if let Some(index) = self.archetypes.get(&archetype) {
                    self.tables[*index].spawn_group(entities);
                } else {
                    if let Some(ent) = selfspawn.get_mut(&archetype) {
                        ent.append(&mut entities);
                    } else {
                        selfspawn.insert(archetype, entities);
                    }
                }
            }
        }
        // queue destroys
        if let Some(ref mut destroy) = queue.destroy {
            while let Some((table, destroy)) = destroy.pop_first() {
                self.tables[table].destroy_group(destroy);
            }
        }
        // queue modifies
        if let Some(ref mut modify) = queue.modify {
            while let Some((index, (insert, remove))) = modify.pop() {
                self.tables[index.table].modify_group(index.col, insert, remove);
            }
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Archetype(pub u64);

impl Archetype {
    pub const fn new() -> Self {
        Self(0)
    }

    pub fn add(&mut self, mut id: u64) {
        id = id.wrapping_mul(123456789123456789);
        self.0 = self.0.wrapping_add(id.wrapping_shl((id % 32) as u32 + 1));
    }

    pub fn clear(&mut self) {
        self.0 = 0
    }
}
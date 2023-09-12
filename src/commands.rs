
use std::collections::BTreeMap;
use std::sync::Arc;

use indexmap::IndexMap;
use strata_traits::Component;

use crate::entity::Entity;
use crate::archetypes::{Archetype, ComponentId};
use crate::anon::Anon;
use crate::archetypes::TableIndex;
use crate::table::DestroyType;
use crate::engine::Engine;
use crate::systems::SystemParam;
use crate::scheduler::Accessor;
use crate::entity::EntityIndex;
use crate::resources::Resources;
use crate::archetypes::Archetypes;
use crate::scheduler::UnsafeRef;

pub struct Commands {
    engine: UnsafeRef<Engine>,
    queue: Queue,
}

impl Commands {
    pub fn spawn<F>(&mut self, predicate: F)
    where
        F: Fn(&mut Entity) 
    {
        let mut entity = Entity::new();
        predicate(&mut entity);
        self.queue.spawn(entity);
    }

    pub fn destroy(&mut self, index: EntityIndex) {
        self.queue.destroy(index);
    }

    pub fn insert<C: Component>(&mut self, index: EntityIndex, cmp: C) {
        self.queue.insert(Anon::new::<C>(cmp), index);
    }

    pub fn remove<C: Component>(&mut self, index: EntityIndex) {
        self.queue.remove(index, C::__internal_id());
    }
}

impl Drop for Commands {
    fn drop(&mut self) {
        self.engine.get().archetypes.queue(std::mem::take(&mut self.queue));
    }
}

impl SystemParam for Commands {
    fn fetch_param(engine: UnsafeRef<Engine>) -> Self {
        Commands {
            engine,
            queue: Queue::default(),
        }
    }

    fn fetch_access() -> Vec<Accessor> {
        vec![Accessor::None]
    }

    fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>) {
        // do nothing
    }
}

pub struct Queue {
    pub spawn: Option<BTreeMap<Archetype, Vec<Entity>>>,
    pub destroy: Option<BTreeMap<TableIndex, Vec<DestroyType>>>,
    pub modify: Option<IndexMap<EntityIndex, (Vec<Anon>, Vec<ComponentId>)>>,
}

impl Queue {
    pub fn spawn(&mut self, mut entity: Entity) {
        entity.hash();

        if let Some(ref mut spawn) = self.spawn {
            if let Some(entities) = spawn.get_mut(&entity.archetype) {
                entities.push(entity);
            } else {
                spawn.insert(entity.archetype, vec![entity]);
            }
        } else {
            let mut spawn = Some(BTreeMap::new());
            spawn.as_mut().unwrap().insert(entity.archetype, vec![entity]);
            self.spawn = spawn;
        }
    }

    pub fn destroy(&mut self, index: EntityIndex) {
        if let Some(ref mut destroy) = self.destroy {
            if let Some(destroys) = destroy.get_mut(&index.table) {
                destroys.push(DestroyType::Drop(index.col));
            }
        } else {
            let mut destroy = Some(BTreeMap::new());
            destroy.as_mut().unwrap().insert(index.table, vec![DestroyType::Drop(index.col)]);
            self.destroy = destroy;
        }
    }

    pub fn insert(&mut self, anon: Anon, index: EntityIndex) {
        if let Some(ref mut modify) = self.modify {
            if let Some((insert, _)) = modify.get_mut(&index) {
                insert.push(anon);
            } else {
                modify.insert(index, (vec![anon], Vec::new()));
            }
        } else {
            let mut modify = Some(IndexMap::new());
            modify.as_mut().unwrap().insert(index, (vec![anon], Vec::new()));
            self.modify = modify;
        }
    }

    pub fn remove(&mut self, index: EntityIndex, id: ComponentId) {
        if let Some(ref mut modify) = self.modify {
            if let Some((_, destroy)) = modify.get_mut(&index) {
                destroy.push(id);
            } else {
                modify.insert(index, (Vec::new(), vec![id]));
            }
        } else {
            let mut modify = Some(IndexMap::new());
            modify.as_mut().unwrap().insert(index, (Vec::new(), vec![id]));
            self.modify = modify;
        }
    }
}

impl Default for Queue {
    fn default() -> Self {
        Queue {
            spawn: None,
            destroy: None,
            modify: None,
        }
    }
}
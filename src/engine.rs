
use std::sync::Arc;

use strata_traits::Resource;

use crate::resources::Resources;
use crate::archetypes::Archetypes;
use crate::systems::Systems;
use crate::scheduler::UnsafeRef;
use crate::systems::Stage;
use crate::systems::IntoSystem;

pub struct Engine {
    pub(crate) resources: Resources,
    pub(crate) archetypes: Archetypes,
    pub(crate) systems: Systems,
}

impl Engine {
    pub(crate) fn new() -> Self {
        Self {
            resources: Resources::new(),
            archetypes: Archetypes::new(),
            systems: Systems::new(),
        }
    }

    pub(crate) fn finalize(&mut self) {
        let mut queries = Vec::new();
        self.systems.get_queries(&mut queries);
        
        while let Some(query) = queries.pop() {
            self.archetypes.add_query(query);
        }
    }

    pub fn execute_startup(&mut self) {
        self.systems.execute_startup(UnsafeRef::new(&self));
        self.archetypes.flush_queues();
    }

    pub fn execute_systems(&mut self) {
        self.systems.execute_systems(UnsafeRef::new(&self));
        self.archetypes.flush_queues();
    }
}

use strata_traits::Resource;

use crate::engine::Engine;
use crate::systems::{IntoSystem, Stage};

pub struct EngineBuilder {
    engine: Engine,
}

impl EngineBuilder {
    pub fn new() -> Self {
        Self {
            engine: Engine::new(),
        }
    }

    pub fn load_resource<R: Resource + Send + Sync>(&mut self, resource: R) -> &mut Self {
        self.engine.resources.insert(resource);
        self
    }

    pub fn load_system<F, P>(&mut self, system: F, stage: Stage) -> &mut Self
    where
        F: IntoSystem<P>, <F as IntoSystem<P>>::System: Sync + Send
    {
        self.engine.systems.load_system(Box::new(system.into_system()), stage);
        self
    }

    pub fn load_startup<F, P>(&mut self, system: F, stage: Stage) -> &mut Self
    where
        F: IntoSystem<P>, <F as IntoSystem<P>>::System: Sync + Send
    {
        self.engine.systems.load_startup(Box::new(system.into_system()), stage);
        self
    }

    pub fn build(mut self) -> Engine {
        self.engine.finalize();
        self.engine
    }
}
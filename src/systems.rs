
use std::sync::Arc;
use std::marker::PhantomData;
use std::collections::HashMap;

use crate::engine::Engine;
use crate::scheduler::Accessor;
use crate::archetypes::ComponentId;
use crate::scheduler::Scheduler;
use crate::scheduler::UnsafeRef;
use crate::resources::Resources;
use crate::archetypes::Archetypes;

pub struct Systems {
    startup: Vec<Scheduler>,
    systems: Vec<Scheduler>,
}

impl Systems {
    pub fn new() -> Self {
        Self {
            startup: vec![
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
            ],
            systems: vec![
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
                Scheduler::new(),
            ],
        }
    }

    pub fn load_startup(&mut self, system: Box<dyn System + Send + Sync>, stage: Stage) {
        self.systems[stage.index()].insert(system)
    }

    pub fn load_system(&mut self, system: Box<dyn System + Send + Sync>, stage: Stage) {
        self.systems[stage.index()].insert(system)
    }

    pub fn execute_startup(&mut self, engine: UnsafeRef<Engine>) {
        for scheduler in self.startup.iter_mut() {
            scheduler.execute(engine.clone());
        }
    }

    pub fn execute_systems(&mut self, engine: UnsafeRef<Engine>) {
        for scheduler in self.systems.iter_mut() {
            scheduler.execute(engine.clone());
        }
    }

    pub fn get_queries(&mut self, queries: &mut Vec<Vec<ComponentId>>) {
        for scheduler in self.startup.iter() {
            scheduler.get_queries(queries);
        }

        for scheduler in self.systems.iter() {
            scheduler.get_queries(queries);
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Core, Early, Main, Late, Render,
}

impl Stage {
    pub fn index(&self) -> usize {
        match self {
            Stage::Core => 0,
            Stage::Early => 1,
            Stage::Main => 2,
            Stage::Late => 3,
            Stage::Render => 4,
        }
    }
}

pub trait System: 'static {
    fn execute(&self, engine: UnsafeRef<Engine>);
    fn accessors(&self) -> Vec<Accessor>;
    fn queries(&self, queries: &mut Vec<Vec<ComponentId>>);
}

/// Convert Thing to System
pub trait IntoSystem<Params> {
    type System: System;

    fn into_system(self) -> Self::System;
}

/// Convert any function with only system params into a system
impl<F, Params: SystemParam> IntoSystem<Params> for F
where
    F: SystemParamFunction<Params>,
{
    type System = FunctionSystem<F, Params>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            system: self,
            params: PhantomData,
        }
    }
}

/// Represent a system with its params
pub struct FunctionSystem<F: 'static, Params: SystemParam> {
    system: F,
    params: PhantomData<Params>,
}

unsafe impl<F: 'static, Params: SystemParam> Send for FunctionSystem<F, Params> {}
unsafe impl<F: 'static, Params: SystemParam> Sync for FunctionSystem<F, Params> {}

/// Make our wrapper be a System
impl<F, Params: SystemParam> System for FunctionSystem<F, Params>
where
    F: SystemParamFunction<Params>,
{
    fn execute(&self, engine: UnsafeRef<Engine>) {
        SystemParamFunction::execute(&self.system, engine);
    }

    fn accessors(&self) -> Vec<Accessor> {
        SystemParamFunction::accessors(&self.system)
    }

    fn queries(&self, queries: &mut Vec<Vec<ComponentId>>) {
        SystemParamFunction::fetch_queries(&self.system, queries)
    }
}

/// Function with only system params
trait SystemParamFunction<Params: SystemParam>: 'static {
    fn execute(&self, engine: UnsafeRef<Engine>);
    fn accessors(&self) -> Vec<Accessor>;
    fn fetch_queries(&self, queries: &mut Vec<Vec<ComponentId>>);
}

/// Marker Trait for parameters of a system function
pub trait SystemParam: 'static {
    fn fetch_param(engine: UnsafeRef<Engine>) -> Self;
    fn fetch_access() -> Vec<Accessor>;
    fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>);
}

macros::impl_system_param_function!(P1);
macros::impl_system_param_function!(P1,P2);
macros::impl_system_param_function!(P1,P2,P3);
macros::impl_system_param_function!(P1,P2,P3,P4);
macros::impl_system_param_function!(P1,P2,P3,P4,P5);
macros::impl_system_param_function!(P1,P2,P3,P4,P5,P6);
macros::impl_system_param_function!(P1,P2,P3,P4,P5,P6,P7);
macros::impl_system_param_function!(P1,P2,P3,P4,P5,P6,P7,P8);
macros::impl_system_param_function!(P1,P2,P3,P4,P5,P6,P7,P8,P9);

pub mod macros {
    #[macro_export]
    macro_rules! impl_system_param_function {
        ($($p:ident),*) => {
            impl<F, $($p),*> SystemParamFunction<($($p),*,)> for F
            where
                F: Fn($($p),*) -> () + 'static,
                $($p: SystemParam),*
            {
                fn execute(&self, engine: UnsafeRef<Engine>) {
                    self($(<$p as SystemParam>::fetch_param(engine.clone())),*)
                }

                fn accessors(&self) -> Vec<Accessor> {
                    let mut out = Vec::new();
                    $(out.append(&mut <$p as SystemParam>::fetch_access());)*

                    out
                }

                fn fetch_queries(&self, queries: &mut Vec<Vec<ComponentId>>) {
                    $(<$p as SystemParam>::fetch_queries(queries);)*
                }
            }

            impl<$($p),*> SystemParam for ($($p),*,)
            where
                $($p: SystemParam),*
            {
                fn fetch_param(engine: UnsafeRef<Engine>) -> Self {
                    (
                        $(<$p as SystemParam>::fetch_param(engine.clone())),*,
                    )
                }

                fn fetch_access() -> Vec<Accessor> {
                    let mut out = Vec::new();
                    $(out.append(&mut <$p as SystemParam>::fetch_access());)*

                    out
                }

                fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>) {
                    $(<$p as SystemParam>::fetch_queries(queries);)*
                }
            }
        }
    }

    pub(crate) use impl_system_param_function;
}
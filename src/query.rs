
use std::sync::Arc;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use indexmap::IndexSet;
use strata_traits::Component;

use crate::anon::AnonIterChain;
use crate::engine::Engine;
use crate::entity::{EntityIndex, EntityIndexChain};
use crate::systems::SystemParam;
use crate::archetypes::ComponentId;
use crate::scheduler::Accessor;
use crate::archetypes::TableIndex;
use crate::archetypes::Archetype;
use crate::archetypes::Archetypes;
use crate::resources::Resources;
use crate::scheduler::UnsafeRef;

pub struct Query<Q: IntoQuery> {
    engine: UnsafeRef<Engine>,
    marker: PhantomData<Q>,
}

// Make Query a System Parameter
impl<Q: IntoQuery> SystemParam for Query<Q> {
    fn fetch_param(engine: UnsafeRef<Engine>) -> Self {
        Self { engine: engine, marker: PhantomData }
    }

    fn fetch_access() -> Vec<Accessor> {
        Q::accessors()
    }

    fn fetch_queries(queries: &mut Vec<Vec<ComponentId>>) {
        Q::queries(queries);
    }
}

// make query become an iterator
impl<Q: IntoQuery> IntoIterator for Query<Q> {
    type Item = < <Q as IntoQuery>::Item as Iterator>::Item;

    type IntoIter = Q::Item;

    fn into_iter(self) -> Self::IntoIter {
        Q::into_query(self.engine)
    }
}

// Trait to make any tuple a Query
pub trait IntoQuery: 'static {
    type Item: Iterator;

    fn into_query(engine: UnsafeRef<Engine>) -> Self::Item;
    fn accessors() -> Vec<Accessor>;
    fn queries(queries: &mut Vec<Vec<ComponentId>>);
}

pub trait QueryParam: 'static {
    type Item: Component;

    fn collect(engine: &Arc<Engine>, ids: &IndexSet<TableIndex>) -> AnonIterChain<Self::Item>;
    fn as_accessor() -> Accessor;
    fn wrap(data: &'static mut Self::Item) -> Self;
}

pub struct Ref<C: Component> {
    inner: &'static C,
}

impl<C: Component> Deref for Ref<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<C: Component> QueryParam for Ref<C> {
    type Item = C;

    fn collect(engine: &Arc<Engine>, ids: &IndexSet<TableIndex>) -> AnonIterChain<Self::Item> {
        engine.archetypes.collect::<C>(ids)
    }

    fn as_accessor() -> Accessor {
        Accessor::Ref(C::__internal_id())
    }

    fn wrap(data: &'static mut Self::Item) -> Self {
        Self {
            inner: data
        }
    }
}

pub struct Mut<C: Component> {
    inner: &'static mut C,
}

impl<C: Component> Deref for Mut<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<C: Component> DerefMut for Mut<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<C: Component> QueryParam for Mut<C> {
    type Item = C;

    fn collect(engine: &Arc<Engine>, ids: &IndexSet<TableIndex>) -> AnonIterChain<Self::Item> {
        engine.archetypes.collect::<C>(ids)
    }

    fn as_accessor() -> Accessor {
        Accessor::Mut(C::__internal_id())
    }

    fn wrap(data: &'static mut Self::Item) -> Self {
        Self {
            inner: data,
        }
    }
}

pub struct Query1<Q1>
where
    Q1: QueryParam
{
    q1: AnonIterChain<Q1::Item>,
    e: EntityIndexChain,
}

impl<Q1> Iterator for Query1<Q1>
where
    Q1: QueryParam
{
    type Item = (Q1, EntityIndex);

    fn next(&mut self) -> Option<Self::Item> {
        if self.q1.iters.len() > 0 {
            Some((
                Q1::wrap(self.q1.next().unwrap()), 
                self.e.next().unwrap()
            ))
        } else {
            None
        }
    }
}

impl<Q1> IntoQuery for (Q1,)
where
    Q1: QueryParam,
{
    type Item = Query1<Q1>;

    fn into_query(engine: UnsafeRef<Engine>) -> Self::Item {
        let mut archetype = Archetype::new();
        archetype.add(Q1::Item::__internal_id());

        let indices = engine.get().archetypes.query(archetype);

        Query1 {
            q1: engine.get().archetypes.collect::<Q1::Item>(indices),
            e: engine.get().archetypes.collect_indices(indices),
        }
    }

    fn accessors() -> Vec<Accessor> {
        vec![Q1::as_accessor()]
    }

    fn queries(queries: &mut Vec<Vec<ComponentId>>) {
        queries.push(vec![Q1::Item::__internal_id()]);
    }
}

macros::impl_query!(Query2,T1,t1,T2,t2);
macros::impl_query!(Query3,T1,t1,T2,t2,T3,t3);
macros::impl_query!(Query4,T1,t1,T2,t2,T3,t3,T4,t4);
macros::impl_query!(Query5,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5);
macros::impl_query!(Query6,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5,T6,t6);
macros::impl_query!(Query7,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5,T6,t6,T7,t7);
macros::impl_query!(Query8,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5,T6,t6,T7,t7,T8,t8);
macros::impl_query!(Query9,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5,T6,t6,T7,t7,T8,t8,T9,t9);
macros::impl_query!(Query10,T1,t1,T2,t2,T3,t3,T4,t4,T5,t5,T6,t6,T7,t7,T8,t8,T9,t9,T10,t10);

pub mod macros {
    macro_rules! impl_query {
        ($t1:ident, $($t2:ident, $t3:ident),*) => {
            pub struct $t1<$($t2),*>
            where
                $($t2: QueryParam),*
            {
                $($t3: AnonIterChain<$t2::Item>),*,
                e: EntityIndexChain,
            }

            impl<$($t2),*> Iterator for $t1<$($t2),*>
            where
                $($t2: QueryParam),*
            {
                type Item = ($($t2),*, EntityIndex);

                fn next(&mut self) -> Option<Self::Item> {
                    if self.t1.iters.len() > 0 {
                        Some((
                            $($t2::wrap(self.$t3.next().unwrap())),*, 
                            self.e.next().unwrap()
                        ))
                    } else {
                        None
                    }
                }
            }

            impl<$($t2),*> IntoQuery for ($($t2),*,)
            where
                $($t2: QueryParam),*
            {
                type Item = $t1<$($t2),*>;

                fn into_query(engine: UnsafeRef<Engine>) -> Self::Item {
                    let mut archetype = Archetype::new();
                    $(archetype.add($t2::Item::__internal_id());)*
                    let indices = engine.get().archetypes.query(archetype);

                    $t1 {
                        $($t3: engine.get().archetypes.collect::<$t2::Item>(indices)),*,
                        e: engine.get().archetypes.collect_indices(indices)
                    }
                }

                fn accessors() -> Vec<Accessor> {
                    vec![$($t2::as_accessor()),*]
                }

                fn queries(queries: &mut Vec<Vec<ComponentId>>) {
                    queries.push(vec![$($t2::Item::__internal_id()),*]);
                }
            }
        }
    }

    pub(crate) use impl_query;
}
use std::{collections::HashMap, marker::PhantomData};

use bevy_app::App;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    error::Result,
    event::Event,
    lifecycle::Insert,
    observer::On,
    resource::Resource,
    system::{Commands, ResMut},
};

type Phantom<T> = PhantomData<fn() -> T>;

pub trait InSpawnPhase {
    type SpawnPhase;

    fn entity(&self) -> Entity;
}

pub trait SpawnPhase {
    type InitialComponent;
}

#[derive(Event)]
pub struct SpawnPhaseCompleted<P> {
    entity: Entity,
    _marker: Phantom<P>,
}

impl<P> SpawnPhaseCompleted<P> {
    fn new(entity: Entity) -> Self {
        Self {
            entity,
            _marker: PhantomData,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

pub trait LozoAppExt {
    fn register_spawn_event<E>(&mut self) -> &mut Self
    where
        E: InSpawnPhase + Event,
        E::SpawnPhase: SpawnPhase,
        <E::SpawnPhase as SpawnPhase>::InitialComponent: Component;
}

impl LozoAppExt for App {
    fn register_spawn_event<E>(&mut self) -> &mut Self
    where
        E: InSpawnPhase + Event,
        E::SpawnPhase: SpawnPhase,
        <E::SpawnPhase as SpawnPhase>::InitialComponent: Component,
    {
        if let Some(mut counter) = self
            .world_mut()
            .get_resource_mut::<DependencyCounters<E::SpawnPhase>>()
        {
            counter.total += 1;
        } else {
            self.world_mut()
                .init_resource::<DependencyCounters<E::SpawnPhase>>();
        }

        self.add_observer(phase_part_complete::<E>)
            .add_observer(register_dependency_counter::<E::SpawnPhase>)
    }
}

#[derive(Resource)]
struct DependencyCounters<P> {
    current: HashMap<Entity, usize>,
    total: usize,
    _marker: Phantom<P>,
}

impl<P> Default for DependencyCounters<P> {
    fn default() -> Self {
        Self {
            current: HashMap::new(),
            total: 1,
            _marker: PhantomData,
        }
    }
}

impl<P> DependencyCounters<P> {
    fn increment(&mut self, lozo_entity: &Entity) -> Result<&mut Self> {
        *self
            .current
            .get_mut(lozo_entity)
            .expect("no counter for lozo entity {lozo_entity}") += 1;

        Ok(self)
    }

    fn finished(&self, lozo_entity: &Entity) -> bool {
        *self
            .current
            .get(lozo_entity)
            .expect("no counter for lozo entity {lozo_entity}")
            == self.total
    }
}

fn phase_part_complete<E>(
    event: On<E>,
    mut counter: ResMut<DependencyCounters<E::SpawnPhase>>,
    mut commands: Commands,
) -> Result
where
    E: Event + InSpawnPhase,
{
    if counter
        .increment(&event.entity())?
        .finished(&event.entity())
    {
        commands.trigger(SpawnPhaseCompleted::<E::SpawnPhase>::new(event.entity()));
        counter.current.remove(&event.entity());
    }

    Ok(())
}

fn register_dependency_counter<P>(
    event: On<Insert, P::InitialComponent>,
    mut dependency_counter: ResMut<DependencyCounters<P>>,
) where
    P: SpawnPhase + 'static,
    P::InitialComponent: Component,
{
    if let Some(count) = dependency_counter.current.get_mut(&event.entity) {
        *count = 0
    } else {
        dependency_counter.current.insert(event.entity, 0);
    }
}

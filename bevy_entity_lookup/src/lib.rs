use std::{collections::HashMap, fmt::Display, hash::Hash};

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
#[cfg(feature = "bevy_elf")]
use bevy_elf::FromDef;
use thiserror::Error;

mod impls;

pub use bevy_entity_lookup_macros::IntoLookedUp;

pub trait IntoLookedUp {
    type LookedUp;

    fn into_looked_up(
        &self,
        lookup: &LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity>;
}

#[derive(Resource, Default)]
pub struct LookupMap(HashMap<EntityId, Entity>);

impl LookupMap {
    pub fn lookup(&self, id: &EntityId) -> Option<&Entity> {
        self.0.get(id)
    }
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(feature = "bevy_elf")]
#[derive(serde::Deserialize, serde::Serialize, FromDef)]
#[cfg(feature = "bevy_elf")]
#[elf(def_type(Self))]
pub struct EntityId(String);

impl Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for EntityId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl EntityId {
    pub fn new(id: String) -> Self {
        Self::from(id)
    }
}

#[derive(Debug, Clone)]
#[cfg(feature = "bevy_elf")]
#[derive(serde::Deserialize, serde::Serialize, FromDef)]
#[cfg(feature = "bevy_elf")]
#[elf(def_type(Self))]
pub struct EntityRef(String);

impl From<String> for EntityRef {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl EntityRef {
    pub fn new(id: String) -> Self {
        Self::from(id)
    }

    pub fn id(&self) -> EntityId {
        EntityId(self.0.clone())
    }
}

pub struct EntityLookupPlugin;

impl Default for EntityLookupPlugin {
    fn default() -> Self {
        Self
    }
}

impl Plugin for EntityLookupPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LookupMap>()
            .add_observer(on_entity_id_added)
            .add_observer(on_entity_id_removed);
    }
}

fn on_entity_id_added(
    event: On<Add, EntityId>,
    mut lookup: ResMut<LookupMap>,
    query: Query<&EntityId>,
) -> Result {
    let id = query.get(event.entity)?;
    lookup.0.insert(id.clone(), event.entity);

    Ok(())
}

fn on_entity_id_removed(
    event: On<Remove, EntityId>,
    ids: Query<&EntityId>,
    mut lookup_map: ResMut<LookupMap>,
) {
    if let Ok(entity_id) = ids.get(event.entity) {
        lookup_map.0.remove(entity_id);
    }
}

#[derive(Error, Debug)]
#[error("Missing Entity \"{0}\"")]
pub struct MissingEntity(EntityId);

use std::{collections::HashMap, hash::Hash};

use bevy_ecs::entity::Entity;

use crate::{IntoLookedUp, MissingEntity};

impl IntoLookedUp for String {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(self.clone())
    }
}

impl IntoLookedUp for usize {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for u8 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for u16 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for u32 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for u64 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for isize {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for i8 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for i16 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for i32 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for i64 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for f32 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for f64 {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for bool {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

impl IntoLookedUp for () {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(*self)
    }
}

#[cfg(feature = "bevy_asset")]
impl<T: bevy_asset::Asset> IntoLookedUp for bevy_asset::Handle<T> {
    type LookedUp = Self;

    fn into_looked_up(
        &self,
        _: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        Ok(self.clone())
    }
}

impl IntoLookedUp for crate::EntityRef {
    type LookedUp = Entity;

    fn into_looked_up(
        &self,
        lookup: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, MissingEntity> {
        let id = self.id();
        lookup.lookup(&id).copied().ok_or_else(|| MissingEntity(id))
    }
}

impl<T: IntoLookedUp> IntoLookedUp for Option<T> {
    type LookedUp = Option<T::LookedUp>;

    fn into_looked_up(
        &self,
        lookup: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, crate::MissingEntity> {
        self.as_ref().map(|t| t.into_looked_up(lookup)).transpose()
    }
}

impl<T: IntoLookedUp> IntoLookedUp for Vec<T> {
    type LookedUp = Vec<T::LookedUp>;

    fn into_looked_up(
        &self,
        lookup: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, crate::MissingEntity> {
        self.iter().map(|t| t.into_looked_up(lookup)).collect()
    }
}

impl<K, T> IntoLookedUp for HashMap<K, T>
where
    K: Hash + Eq + Clone,
    T: IntoLookedUp,
{
    type LookedUp = HashMap<K, T::LookedUp>;

    fn into_looked_up(
        &self,
        lookup: &crate::LookupMap,
    ) -> std::result::Result<Self::LookedUp, crate::MissingEntity> {
        self.iter()
            .map(|(k, v)| Ok((k.clone(), v.into_looked_up(lookup)?)))
            .collect()
    }
}

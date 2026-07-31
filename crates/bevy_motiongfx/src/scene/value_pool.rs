//! [`Backend`](crate::scene::backend::Backend)'s
//! [`SceneBackend::ValuePool`]: one [`IndexMap`] column per concrete
//! value type a `Transform` animation needs.

use bevy_asset::uuid::Uuid;
use bevy_math::{Quat, Vec3};
use bevy_platform::hash::FixedHasher;
use indexmap::IndexMap;
use motiongfx_scene::prelude::*;
use serde::{Deserialize, Serialize};

/// Columns are insertion-ordered, so a saved scene's values keep the
/// order they were authored in. The hasher is named explicitly because
/// [`IndexMap`]'s default needs indexmap's `std` feature.
#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
pub struct ValuePool {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub f32: IndexMap<Uuid, f32, FixedHasher>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub vec3: IndexMap<Uuid, Vec3, FixedHasher>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub quat: IndexMap<Uuid, Quat, FixedHasher>,
}

macro_rules! impl_value_column {
    ($ty:ty, $field:ident) => {
        impl ValueColumn<Uuid, $ty> for ValuePool {
            fn get(&self, id: Uuid) -> Option<&$ty> {
                self.$field.get(&id)
            }

            fn get_mut(&mut self, id: Uuid) -> Option<&mut $ty> {
                self.$field.get_mut(&id)
            }

            fn insert(&mut self, value: $ty) -> Uuid {
                let id = Uuid::new_v4();
                self.$field.insert(id, value);
                id
            }
        }
    };
}

impl_value_column!(f32, f32);
impl_value_column!(Vec3, vec3);
impl_value_column!(Quat, quat);

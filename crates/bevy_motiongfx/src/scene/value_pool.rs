//! [`Backend`](crate::scene::backend::Backend)'s
//! [`SceneBackend::ValuePool`]: one `SparseMap` column per concrete
//! value type a `Transform` animation needs.

use bevy_math::{Quat, Vec3};
use motiongfx_scene::prelude::*;
use serde::{Deserialize, Serialize};
use sparse_map::{Key, SparseMap};

/// The value key of [`ValuePool`].
pub type ValueId = Key;

#[derive(
    Default, Debug, Clone, PartialEq, Serialize, Deserialize,
)]
pub struct ValuePool {
    #[serde(default, skip_serializing_if = "SparseMap::is_empty")]
    pub f32: SparseMap<f32>,
    #[serde(default, skip_serializing_if = "SparseMap::is_empty")]
    pub vec3: SparseMap<Vec3>,
    #[serde(default, skip_serializing_if = "SparseMap::is_empty")]
    pub quat: SparseMap<Quat>,
}

macro_rules! impl_value_column {
    ($ty:ty, $field:ident) => {
        impl ValueColumn<ValueId, $ty> for ValuePool {
            fn get(&self, id: ValueId) -> Option<&$ty> {
                self.$field.get(&id)
            }

            fn get_mut(&mut self, id: ValueId) -> Option<&mut $ty> {
                self.$field.get_mut(&id)
            }

            fn insert(&mut self, value: $ty) -> ValueId {
                self.$field.insert(value)
            }
        }
    };
}

impl_value_column!(f32, f32);
impl_value_column!(Vec3, vec3);
impl_value_column!(Quat, quat);

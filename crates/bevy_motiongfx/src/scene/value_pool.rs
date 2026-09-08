//! [`Backend`](crate::scene::backend::Backend)'s
//! [`SceneBackend::ValuePool`]: one [`IndexMap`] column per concrete
//! value type a `Transform` animation needs.

use bevy_app::App;
use bevy_asset::uuid::Uuid;
use bevy_math::{Quat, Vec3};
use bevy_platform::hash::FixedHasher;
use bevy_reflect::{FromReflect, FromType, PartialReflect, TypeRegistry};
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

/// [`ValuePool`]'s columns, from one list: a [`ValueColumn`] impl each
/// and [`register_scene_values`], so a new column cannot be added to
/// one without the other.
macro_rules! value_columns {
    ($($ty:ty = $field:ident),* $(,)?) => {
        $(
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
        )*

        /// Registers a [`ReflectSceneValue`] for every [`ValuePool`]
        /// column type, so [`insert_scene_value`] can route a reflected
        /// value to its column.
        pub fn register_scene_values(app: &mut App) -> &mut App {
            app$(
                .register_type::<$ty>()
                .register_type_data::<$ty, ReflectSceneValue>()
            )*
        }
    };
}

value_columns!(f32 = f32, Vec3 = vec3, Quat = quat);

/// Type data for inserting a reflected `T` into its [`ValuePool`]
/// column. Registered by [`register_scene_values`], looked up by
/// [`insert_scene_value`].
#[derive(Clone)]
pub struct ReflectSceneValue {
    insert: fn(&mut ValuePool, &dyn PartialReflect) -> Option<Uuid>,
}

impl ReflectSceneValue {
    /// Inserts `value` into the column for this data's type, returning
    /// the id to reach it again, or `None` if `value` is not that type.
    pub fn insert(
        &self,
        pool: &mut ValuePool,
        value: &dyn PartialReflect,
    ) -> Option<Uuid> {
        (self.insert)(pool, value)
    }
}

impl<T> FromType<T> for ReflectSceneValue
where
    T: FromReflect,
    ValuePool: ValueColumn<Uuid, T>,
{
    fn from_type() -> Self {
        Self {
            insert: |pool, value| {
                let value = T::from_reflect(value)?;
                Some(<ValuePool as ValueColumn<Uuid, T>>::insert(
                    pool, value,
                ))
            },
        }
    }
}

/// Inserts `value` into the [`ValuePool`] column for its type, or
/// `None` if no [`ReflectSceneValue`] is registered for it.
///
/// Keyed on the value's represented type, so a `Quat`'s
/// `x`/`y`/`z`-shaped dynamic is never taken for a `Vec3`.
pub fn insert_scene_value(
    pool: &mut ValuePool,
    registry: &TypeRegistry,
    value: &dyn PartialReflect,
) -> Option<Uuid> {
    let type_id = value.get_represented_type_info()?.type_id();
    registry
        .get_type_data::<ReflectSceneValue>(type_id)?
        .insert(pool, value)
}

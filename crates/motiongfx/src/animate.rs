use bevy::asset::AsAssetId;
use bevy::ecs::component::Mutable;
use bevy::prelude::*;

use crate::field::{FieldBundle, RegisterFieldAppExt};
use crate::interpolation::Interpolation;
use crate::sequence::segment::{
    bake_asset_actions, bake_component_actions,
    sample_asset_keyframes, sample_component_keyframes,
};
use crate::{MotionGfxSet, ThreadSafe};

// TODO: Add a macro or something* to register multiple
// animatable fields from a single struct at once.

pub trait AnimateAppExt {
    fn animate_component<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: Component<Mutability = Mutable>,
        Target: Interpolation + Clone + ThreadSafe;

    fn animate_asset<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source::Asset, Target>,
    ) -> &mut Self
    where
        Source: AsAssetId,
        Target: Interpolation + Clone + ThreadSafe;
}

impl AnimateAppExt for App {
    fn animate_component<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source, Target>,
    ) -> &mut Self
    where
        Source: Component<Mutability = Mutable>,
        Target: Interpolation + Clone + ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            sample_component_keyframes(field_bundle.field)
                .in_set(MotionGfxSet::Sample),
        )
        .add_observer(bake_component_actions(field_bundle.field))
        .register_field(field_bundle)
    }

    fn animate_asset<Source, Target>(
        &mut self,
        field_bundle: FieldBundle<Source::Asset, Target>,
    ) -> &mut Self
    where
        Source: AsAssetId,
        Target: Interpolation + Clone + ThreadSafe,
    {
        self.add_systems(
            PostUpdate,
            sample_asset_keyframes::<Source, _>(field_bundle.field)
                .in_set(MotionGfxSet::Sample),
        )
        .add_observer(bake_asset_actions::<Source, _>(
            field_bundle.field,
        ))
        .register_field(field_bundle)
    }
}

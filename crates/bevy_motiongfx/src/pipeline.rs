use bevy_app::prelude::*;
#[cfg(feature = "asset")]
use bevy_asset::Asset;
use bevy_ecs::component::Mutable;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

use crate::MotionGfxSet;

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                bake_timeline.in_set(MotionGfxSet::Bake),
                queue_timeline.in_set(MotionGfxSet::QueueAction),
                sample_timeline.in_set(MotionGfxSet::Sample),
            ),
        )
        .add_observer(mark_bake_timeline);
    }
}

pub fn bake_component_actions<S, T>(ctx: BakeCtx)
where
    S: Component,
    T: Clone + ThreadSafe,
{
    ctx.bake::<Entity, S, T>(|entity, target_world, accessor| {
        target_world.get::<S>(entity).map(|s| (accessor.ref_fn)(s))
    });
}

pub fn sample_component_actions<S, T>(ctx: SampleCtx)
where
    S: Component<Mutability = Mutable>,
    T: Clone + ThreadSafe,
{
    ctx.sample::<Entity, S, T>(
        |target, entity, target_world, accessor| {
            if let Some(mut source) =
                target_world.get_mut::<S>(entity)
            {
                *(accessor.mut_fn)(&mut source) = target;
            }

            target_world
        },
    );
}

#[cfg(feature = "asset")]
pub fn bake_asset_actions<S, T>(ctx: BakeCtx)
where
    S: Asset,
    T: Clone + ThreadSafe,
{
    use bevy_asset::Assets;
    use bevy_asset::UntypedAssetId;

    ctx.bake::<UntypedAssetId, S, T>(
        |asset_id, target_world, accessor| {
            target_world
                .get_resource::<Assets<S>>()?
                .get(asset_id.typed::<S>())
                .map(|s| (accessor.ref_fn)(s))
        },
    );
}

#[cfg(feature = "asset")]
pub fn sample_asset_actions<S, T>(ctx: SampleCtx)
where
    S: Asset,
    T: Clone + ThreadSafe,
{
    use bevy_asset::Assets;
    use bevy_asset::UntypedAssetId;

    ctx.sample::<UntypedAssetId, S, T>(
        |target, asset_id, target_world, accessor| {
            if let Some(mut assets) =
                target_world.get_resource_mut::<Assets<S>>()
            {
                if let Some(source) =
                    assets.get_mut(asset_id.typed::<S>())
                {
                    *(accessor.mut_fn)(source) = target;
                }
            }

            target_world
        },
    );
}

pub trait PipelineRegistryExt {
    fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe;

    #[cfg(feature = "asset")]
    fn register_asset<S, T>(&mut self) -> PipelineKey
    where
        S: Asset,
        T: Clone + ThreadSafe;
}

impl PipelineRegistryExt for PipelineRegistry {
    fn register_component<S, T>(&mut self) -> PipelineKey
    where
        S: Component<Mutability = Mutable>,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<S, T>();

        self.register_unchecked(
            key,
            Pipeline::new(
                bake_component_actions::<S, T>,
                sample_component_actions::<S, T>,
            ),
        );

        key
    }

    #[cfg(feature = "asset")]
    fn register_asset<S, T>(&mut self) -> PipelineKey
    where
        S: Asset,
        T: Clone + ThreadSafe,
    {
        let key = PipelineKey::new::<S, T>();

        self.register_unchecked(
            key,
            Pipeline::new(
                bake_asset_actions::<S, T>,
                sample_asset_actions::<S, T>,
            ),
        );

        key
    }
}

// TODO: Optimize samplers into parallel operations.
// This could be deferred into motiongfx::pipeline?
// See also https://github.com/voxell-tech/motiongfx/issues/72

fn mark_bake_timeline(
    trigger: On<Insert, Timeline>,
    mut commands: Commands,
) {
    commands.entity(trigger.entity).insert(BakeTimeline);
}

/// # Panics
///
/// Panics if the [`Timeline`] component is baking itself.
fn bake_timeline(main_world: &mut World) {
    let mut q_timelines = main_world
        .query_filtered::<(&mut Timeline, Entity), Added<BakeTimeline>>();
    let main_cell = main_world.as_unsafe_world_cell();

    // SAFETY: Timeline should never bake timeline itself.
    unsafe {
        let pipeline_registry =
            main_cell.get_resource::<PipelineRegistry>().expect(
                "`PipelineRegistry` resource should be inserted.",
            );

        let accessor_registry =
        main_cell.get_resource::<FieldAccessorRegistry>().expect(
            "`FieldAccessorRegistry` resource should be inserted.",
        );

        let mut commands = main_cell.world_mut().commands();

        for (mut timeline, entity) in
            q_timelines.iter_mut(main_cell.world_mut())
        {
            timeline.bake_actions(
                pipeline_registry,
                main_cell.world(),
                accessor_registry,
            );

            commands.entity(entity).remove::<BakeTimeline>();
        }
    }
}

fn queue_timeline(
    mut q_timelines: Query<&mut Timeline, Changed<Timeline>>,
) {
    for mut timeline in q_timelines.iter_mut() {
        let timeline = timeline.bypass_change_detection();
        timeline.queue_actions();
    }
}

/// # Panics
///
/// Panics if the [`Timeline`] component is sampling itself.
fn sample_timeline(main_world: &mut World) {
    let mut q_timelines =
        main_world.query_filtered::<&Timeline, Changed<Timeline>>();

    let main_cell = main_world.as_unsafe_world_cell();

    // SAFETY: Timeline should never sample timeline itself.
    unsafe {
        let pipeline_registry =
            main_cell.get_resource::<PipelineRegistry>().expect(
                "`PipelineRegistry` resource should be inserted.",
            );

        let accessor_registry =
        main_cell.get_resource::<FieldAccessorRegistry>().expect(
            "`FieldAccessorRegistry` resource should be inserted.",
        );

        for timeline in q_timelines.iter(main_cell.world()) {
            timeline.sample_queued_actions(
                pipeline_registry,
                main_cell.world_mut(),
                accessor_registry,
            );
        }
    }
}

/// Marker component for timelines to be baked. This will be inserted
/// automatically on [`Timeline`] insertion trigger and removed after
/// the baking process is completed.
#[derive(Component)]
struct BakeTimeline;

//! Baking is the process of realizing [`Action`]s into solid
//! [`Segment`] values.

use bevy::asset::AsAssetId;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::action::Action;
use crate::field::{DynField, Field, FieldRegistry};
use crate::timeline_v2::Timeline;
use crate::ThreadSafe;

pub struct BakePlugin;

impl Plugin for BakePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_insert_timeline);
    }
}

pub trait BakeAppExt {
    fn bake_component<Source, Target>(
        &mut self,
        field: Field<Source, Target>,
    ) where
        Source: Component,
        Target: Clone + ThreadSafe;

    #[cfg(feature = "asset")]
    fn bake_asset<Source, Target>(
        &mut self,
        field: Field<Source::Asset, Target>,
    ) where
        Source: AsAssetId,
        Target: Clone + ThreadSafe;
}

impl BakeAppExt for App {
    fn bake_component<Source, Target>(
        &mut self,
        field: Field<Source, Target>,
    ) where
        Source: Component,
        Target: Clone + ThreadSafe,
    {
        self.add_observer(
            on_bake_timeline(field).pipe(bake_component_actions),
        );
    }

    #[cfg(feature = "asset")]
    fn bake_asset<Source, Target>(
        &mut self,
        field: Field<Source::Asset, Target>,
    ) where
        Source: AsAssetId,
        Target: Clone + ThreadSafe,
    {
        self.add_observer(
            on_bake_timeline(field)
                .pipe(bake_asset_actions::<Source, _>),
        );
    }
}

#[derive(Event)]
pub struct BakeTimeline;

/// Trigger the bake event on insert.
fn on_insert_timeline(
    trigger: Trigger<OnInsert, Timeline>,
    mut commands: Commands,
) {
    commands.trigger_targets(BakeTimeline, trigger.target());
}

fn on_bake_timeline<Source: 'static, Target: 'static>(
    field: Field<Source, Target>,
) -> impl Fn(Trigger<BakeTimeline, Timeline>) -> BakeInput<Source, Target>
{
    // Precompute the hash!
    let hash = field.untyped();
    move |trigger: Trigger<BakeTimeline, Timeline>| BakeInput {
        timeline_target: trigger.target(),
        dyn_field: DynField::from_hash(hash),
    }
}

fn bake_component_actions<Source, Target>(
    input: In<BakeInput<Source, Target>>,
    mut bake_param: BakeActionParam<Target>,
    q_components: Query<&Source>,
) -> Result
where
    Source: Component,
    Target: Clone + ThreadSafe,
{
    bake_param
        .bake_actions(&input, |entity| q_components.get(entity).ok())
}

#[cfg(feature = "asset")]
fn bake_asset_actions<Source, Target>(
    input: In<BakeInput<Source::Asset, Target>>,
    mut bake_param: BakeActionParam<Target>,
    q_components: Query<&Source>,
    assets: Res<Assets<Source::Asset>>,
) -> Result
where
    Source: AsAssetId,
    Target: Clone + ThreadSafe,
{
    // let input = BakeInput {
    //     timeline_target: input.timeline_target,
    //     dyn_field: DynField::from_hash(*input.dyn_field.hash_ref()),
    // };

    bake_param.bake_actions(&input, |entity| {
        q_components
            .get(entity)
            .ok()
            .and_then(|c| assets.get(c.as_asset_id()))
    })
}

struct BakeInput<Source, Target> {
    timeline_target: Entity,
    dyn_field: DynField<Source, Target>,
}

#[derive(SystemParam)]
struct BakeActionParam<'w, 's, Target>
where
    Target: 'static,
{
    commands: Commands<'w, 's>,
    q_timelines: Query<'w, 's, &'static Timeline>,
    q_actions: Query<'w, 's, &'static Action<Target>>,
    registry: Res<'w, FieldRegistry>,
}

impl<Target> BakeActionParam<'_, '_, Target>
where
    Target: Clone + ThreadSafe,
{
    pub fn bake_actions<'a, Source: 'static>(
        &mut self,
        input: &BakeInput<Source, Target>,
        source_ref: impl Fn(Entity) -> Option<&'a Source>,
    ) -> Result {
        let hash = input.dyn_field.hash_ref();
        let timeline = self.q_timelines.get(input.timeline_target)?;

        for track in timeline.tracks() {
            for (key, spans) in track.iter_sequences() {
                // Safely skip if field hash is not the same.
                if key.field() != hash {
                    continue;
                }

                let accessor = self
                    .registry
                    .get_accessor::<Source, Target>(*hash)
                    .ok_or(format!(
                        "No `FieldAccessor` for {hash:?}"
                    ))?;

                // Get a reference to the source.
                let source_err = || {
                    format!(
                        "Unable to get source for {:?} from {}",
                        hash,
                        key.target()
                    )
                };
                let source = source_ref(key.target())
                    .ok_or_else(source_err)?;

                // Clone the target field from the source.
                let mut value = accessor.get_ref(source).clone();

                for span in spans {
                    let action_id = span.action_id();
                    let action = self.q_actions.get(action_id)?;

                    // Update field to the next value using action.
                    let end_value = action(&value);

                    self.commands.entity(action_id).insert(
                        Segment::new(value, end_value.clone()),
                    );

                    value = end_value;
                }
            }
        }

        Ok(())
    }
}

#[derive(Component)]
pub struct Segment<T> {
    /// The starting value in the segment.
    start: T,
    /// The ending value in the segment.
    end: T,
}

impl<T> Segment<T> {
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> &T {
        &self.start
    }

    pub fn end(&self) -> &T {
        &self.end
    }
}

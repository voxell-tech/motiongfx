use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use motiongfx::prelude::*;

use crate::MotionGfxSet;

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                queue_timeline.in_set(MotionGfxSet::QueueAction),
                sample_timeline.in_set(MotionGfxSet::Sample),
            ),
        )
        .add_observer(bake_timeline);
    }
}

// TODO: Optimize samplers into parallel operations.
// This could be deferred into motiongfx::pipeline?
// See also https://github.com/voxell-tech/motiongfx/issues/72

/// # Panics
///
/// Panics if the [`Timeline`] component is baking itself.
fn bake_timeline(
    trigger: Trigger<OnInsert, Timeline>,
    main_world: &mut World,
) {
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

        let mut timeline = main_cell
            .get_entity(trigger.target())
            .unwrap()
            .get_mut::<Timeline>()
            .unwrap();

        timeline.bake_actions(
            pipeline_registry,
            main_cell.world(),
            accessor_registry,
        );
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

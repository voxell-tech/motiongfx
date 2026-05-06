use core::ops::{Deref, DerefMut};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use motiongfx::prelude::*;

use crate::MotionGfxSet;
use crate::controller::FixedRatePlayer;
use crate::controller::RealtimePlayer;
use crate::prelude::BevyTimelineBuilder;
use crate::world::{BevyTimeline, BevyWorld};

pub struct MotionGfxManagerPlugin;

impl Plugin for MotionGfxManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MotionGfxManager>().add_systems(
            PostUpdate,
            (
                sample_timelines.in_set(MotionGfxSet::Sample),
                (
                    complete_timelines::<RealtimePlayer>,
                    complete_timelines::<FixedRatePlayer>,
                )
                    .after(MotionGfxSet::Sample),
            ),
        );
    }
}

// TODO: Optimize samplers into parallel operations.
// This could be deferred into motiongfx::pipeline?
// See also https://github.com/voxell-tech/motiongfx/issues/72

/// # Panics
///
/// Panics if the [`Timeline`] component is sampling itself.
fn sample_timelines(world: &mut World) {
    world.try_resource_scope::<MotionGfxManager, _>(
        |world, mut motiongfx| {
            motiongfx.load_pending_timelines(world);
            motiongfx.sample_timelines(world);
        },
    );
}

/// A unique Id for a [`Timeline`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineId(u64);

/// Signal for complete timelines
#[derive(Component)]
pub struct TimelineComplete;

#[allow(clippy::type_complexity)]
fn complete_timelines<T>(
    mut commands: Commands,
    motiongfx: Res<MotionGfxManager>,
    timelines: Query<
        (Entity, &TimelineId),
        (With<T>, Without<TimelineComplete>),
    >,
) where
    T: Component,
{
    for (entity, timeline) in timelines.iter() {
        if motiongfx
            .get_timeline(timeline)
            .is_some_and(|t| t.is_complete())
        {
            commands.entity(entity).insert(TimelineComplete);
        }
    }
}

/// Resources that the [`motiongfx`] framework operates on.
#[derive(Resource)]
pub struct MotionGfxManager {
    id: TimelineId,
    pending_timelines: HashMap<TimelineId, MutDetect<BevyTimeline>>,
    timelines: HashMap<TimelineId, MutDetect<BevyTimeline>>,
    registry: Registry,
}

impl Default for MotionGfxManager {
    fn default() -> Self {
        Self {
            id: TimelineId(0),
            pending_timelines: Default::default(),
            timelines: Default::default(),
            registry: Default::default(),
        }
    }
}

impl MotionGfxManager {
    pub fn create_builder(&mut self) -> BevyTimelineBuilder<'_> {
        TimelineBuilder::new(&mut self.registry)
    }

    pub fn add_timeline(
        &mut self,
        timeline: BevyTimeline,
    ) -> TimelineId {
        let id = self.id;
        self.pending_timelines.insert(id, MutDetect::new(timeline));

        self.id.0 = self.id.0.wrapping_add(1);
        id
    }

    pub fn remove_timeline(
        &mut self,
        id: &TimelineId,
    ) -> Option<BevyTimeline> {
        self.timelines
            .remove(id)
            .or_else(|| self.pending_timelines.remove(id))
            .map(|t| t.take())
    }

    pub fn get_timeline(
        &self,
        id: &TimelineId,
    ) -> Option<&BevyTimeline> {
        self.timelines
            .get(id)
            .or_else(|| self.pending_timelines.get(id))
            .map(|t| &**t)
    }

    pub fn get_timeline_mut(
        &mut self,
        id: &TimelineId,
    ) -> Option<&mut MutDetect<BevyTimeline>> {
        self.timelines
            .get_mut(id)
            .or_else(|| self.pending_timelines.get_mut(id))
    }

    pub fn load_pending_timelines(&mut self, world: &World) {
        for (id, mut timeline) in self.pending_timelines.drain() {
            timeline.bake_actions(
                &self.registry,
                BevyWorld::from_ref(world),
            );
            self.timelines.insert(id, timeline);
        }
    }

    pub fn sample_timelines(&mut self, world: &mut World) {
        for timeline in
            self.timelines.values_mut().filter(|t| t.mutated())
        {
            timeline.queue_actions();
            timeline.sample_queued_actions(
                &self.registry,
                BevyWorld::from_mut(world),
            );
            timeline.reset();
        }
    }
}

pub struct MutDetect<T> {
    inner: T,
    mutated: bool,
}

impl<T> MutDetect<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            mutated: false,
        }
    }

    pub fn mutated(&self) -> bool {
        self.mutated
    }

    /// Reset mutation detection flag to `false`.
    pub fn reset(&mut self) {
        self.mutated = false
    }

    pub fn take(self) -> T {
        self.inner
    }
}

impl<T> Deref for MutDetect<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for MutDetect<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        &mut self.inner
    }
}

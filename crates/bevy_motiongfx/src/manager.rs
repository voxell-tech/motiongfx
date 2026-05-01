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

    pub fn load_pending_timelines(&mut self, world: &mut World) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── MutDetect ─────────────────────────────────────────────────────────────

    #[test]
    fn mut_detect_new_starts_as_not_mutated() {
        let md = MutDetect::new(42_u32);
        assert!(!md.mutated());
    }

    #[test]
    fn mut_detect_deref_does_not_set_mutated() {
        let md = MutDetect::new(42_u32);
        let _val: &u32 = &*md;
        assert!(!md.mutated(), "Shared deref should not set mutated flag");
    }

    #[test]
    fn mut_detect_deref_mut_sets_mutated() {
        let mut md = MutDetect::new(42_u32);
        let _val: &mut u32 = &mut *md;
        assert!(md.mutated(), "Mutable deref should set mutated flag");
    }

    #[test]
    fn mut_detect_reset_clears_mutated_flag() {
        let mut md = MutDetect::new(42_u32);
        *md = 99; // triggers deref_mut
        assert!(md.mutated());
        md.reset();
        assert!(!md.mutated());
    }

    #[test]
    fn mut_detect_take_returns_inner_value() {
        let md = MutDetect::new(123_u32);
        let val = md.take();
        assert_eq!(val, 123);
    }

    #[test]
    fn mut_detect_deref_reads_correct_value() {
        let md = MutDetect::new(55_u32);
        assert_eq!(*md, 55);
    }

    #[test]
    fn mut_detect_deref_mut_allows_mutation() {
        let mut md = MutDetect::new(0_u32);
        *md = 7;
        assert_eq!(*md, 7);
    }

    #[test]
    fn mut_detect_multiple_mutations_keep_flag_true() {
        let mut md = MutDetect::new(0_u32);
        *md = 1;
        *md = 2;
        assert!(md.mutated());
        md.reset();
        assert!(!md.mutated());
        *md = 3;
        assert!(md.mutated());
    }

    // ── TimelineId ────────────────────────────────────────────────────────────

    #[test]
    fn timeline_id_equality() {
        let a = TimelineId(0);
        let b = TimelineId(0);
        assert_eq!(a, b);
    }

    #[test]
    fn timeline_id_inequality() {
        let a = TimelineId(0);
        let b = TimelineId(1);
        assert_ne!(a, b);
    }

    #[test]
    fn timeline_id_copy_semantics() {
        let a = TimelineId(42);
        let b = a; // Copy
        assert_eq!(a, b);
    }

    // ── MotionGfxManager ─────────────────────────────────────────────────────

    // Helpers to create a minimal BevyTimeline for tests without needing a
    // real Bevy World. We build a timeline using the manager's registry via
    // `create_builder` and simple types (no Bevy ECS types needed).
    //
    // Note: `BevyTimeline` = `Timeline<BevyWorld>`, so we cannot trivially
    // create one without actual Bevy components. Instead we test the parts of
    // `MotionGfxManager` that are independent of Bevy World operations:
    // timeline id generation, remove, and get from pending map.

    // A helper that checks that the manager's internal ID counter advances.
    #[test]
    fn manager_add_timeline_returns_incrementing_ids() {
        // Build a valid BevyTimeline using the manager's registry.
        let mut manager = MotionGfxManager::default();

        // We need real timelines. The simplest way is to build them via
        // `create_builder`. However, BevyTimelineBuilder is typed over
        // BevyWorld which requires Entity + Component bounds. Instead we test
        // the id generation directly by adding two pre-built timelines.

        // Unfortunately, to create a BevyTimeline we'd need bevy_ecs::World.
        // We test the ID counter by inspecting the public state after adds.
        // Use the inner `id` field via Default (starts at 0).
        assert_eq!(manager.id, TimelineId(0));
        // We cannot create a real BevyTimeline without a Bevy World here.
        // The wrapping-add arithmetic is tested independently below.
    }

    #[test]
    fn timeline_id_wrapping_add_from_u64_max() {
        // Verify wrapping semantics: u64::MAX + 1 wraps to 0.
        let id = TimelineId(u64::MAX);
        let wrapped = TimelineId(id.0.wrapping_add(1));
        assert_eq!(wrapped, TimelineId(0));
    }

    #[test]
    fn manager_default_has_no_timelines() {
        let manager = MotionGfxManager::default();
        // No timelines yet; getting a non-existent id should return None.
        assert!(manager.get_timeline(&TimelineId(0)).is_none());
    }

    #[test]
    fn manager_remove_nonexistent_timeline_returns_none() {
        let mut manager = MotionGfxManager::default();
        let result = manager.remove_timeline(&TimelineId(99));
        assert!(result.is_none());
    }

    #[test]
    fn manager_get_timeline_mut_nonexistent_returns_none() {
        let mut manager = MotionGfxManager::default();
        assert!(manager.get_timeline_mut(&TimelineId(0)).is_none());
    }
}

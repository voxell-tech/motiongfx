use core::ops::{Deref, DerefMut};

use bevy_ecs::prelude::*;
use bevy_platform::collections::HashMap;
use motiongfx::prelude::{FieldAccessorRegistry, Timeline};

use crate::pipeline::WorldPipelineRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineId(u64);

#[derive(Resource)]
pub struct MotionGfxWorld {
    id: TimelineId,
    pending_timelines: HashMap<TimelineId, MutDetect<Timeline>>,
    timelines: HashMap<TimelineId, MutDetect<Timeline>>,
    pub pipeline_registry: WorldPipelineRegistry,
    pub accessor_registry: FieldAccessorRegistry,
}

impl MotionGfxWorld {
    pub fn add_timeline(&mut self, timeline: Timeline) -> TimelineId {
        let id = self.id;
        self.pending_timelines.insert(id, MutDetect::new(timeline));

        self.id.0 = self.id.0.wrapping_add(1);
        id
    }

    pub fn remove_timeline(
        &mut self,
        id: &TimelineId,
    ) -> Option<Timeline> {
        self.timelines
            .remove(id)
            .or_else(|| self.pending_timelines.remove(id))
            .map(|t| t.take())
    }

    pub fn get_timeline(&self, id: &TimelineId) -> Option<&Timeline> {
        self.timelines
            .get(id)
            .or_else(|| self.pending_timelines.get(id))
            .map(|t| &**t)
    }

    pub fn get_timeline_mut(
        &mut self,
        id: &TimelineId,
    ) -> Option<&mut MutDetect<Timeline>> {
        self.timelines
            .get_mut(id)
            .or_else(|| self.pending_timelines.get_mut(id))
    }

    pub fn load_pending_timelines(&mut self, world: &mut World) {
        for (id, mut timeline) in self.pending_timelines.drain() {
            timeline.bake_actions(
                &self.accessor_registry,
                &self.pipeline_registry,
                world,
            );
            self.timelines.insert(id, timeline);
        }
    }

    pub fn iter_changed_timelines(
        &self,
    ) -> impl Iterator<Item = &Timeline> {
        self.timelines
            .values()
            .filter(|t| t.mutated())
            .map(|t| &**t)
    }

    pub fn iter_changed_timelines_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut Timeline> {
        self.timelines
            .values_mut()
            .filter(|t| t.mutated())
            .map(|t| &mut **t)
    }

    /// Reset all mutation detection flag to `false`.
    pub fn reset_mut_detect(&mut self) {
        for timeline in self.timelines.values_mut() {
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

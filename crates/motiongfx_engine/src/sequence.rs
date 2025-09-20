use nonempty::NonEmpty;

use crate::action::ActionClip;

/// A non-overlapping sequence of [`ActionClip`]s.
#[derive(Debug, Clone)]
pub struct Sequence {
    pub clips: NonEmpty<ActionClip>,
}

impl Sequence {
    pub const fn new(span: ActionClip) -> Self {
        Self {
            clips: NonEmpty::new(span),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Get the offset time of the sequence.
    #[inline]
    pub fn offset(&self) -> f32 {
        // TODO: Change naming
        self.clips.first().start
    }

    /// Get the end time of the sequence.
    #[inline]
    pub fn end(&self) -> f32 {
        self.clips.last().end()
    }

    /// Get the duration of the sequence.
    #[inline]
    pub fn duration(&self) -> f32 {
        self.end() - self.offset()
    }

    pub(crate) fn delay(&mut self, duration: f32) {
        for clip in self.clips.iter_mut() {
            clip.start += duration;
        }
    }
}

impl Sequence {
    #[inline]
    pub fn push(&mut self, span: ActionClip) {
        debug_assert!(
            span.start >= self.end(),
            "({} >= {}) `ActionClip`s shouldn't overlap!",
            span.start,
            self.end(),
        );

        self.clips.push(span);
    }
}

impl Extend<ActionClip> for Sequence {
    #[inline]
    fn extend<T: IntoIterator<Item = ActionClip>>(
        &mut self,
        iter: T,
    ) {
        #[cfg(debug_assertions)]
        let mut end = self.end();
        #[cfg(debug_assertions)]
        let iter = {
            iter.into_iter().map(|clip| {
                debug_assert!(
                    clip.start >= end,
                    "({} >= {}) `ActionClip`s shouldn't overlap!",
                    clip.start,
                    end,
                );

                end = clip.end();
                clip
            })
        };

        self.clips.extend(iter);
    }
}

impl IntoIterator for Sequence {
    type Item = ActionClip;

    type IntoIter = <NonEmpty<ActionClip> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.clips.into_iter()
    }
}

// pub mod action {
//     use bevy::prelude::*;

//     #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
//     pub struct ActionId(Entity);

//     impl ActionId {
//         pub const PLACEHOLDER: Self = Self(Entity::PLACEHOLDER);

//         pub(crate) fn new(entity: Entity) -> Self {
//             Self(entity)
//         }

//         pub(crate) fn inner(&self) -> Entity {
//             self.0
//         }
//     }

//     #[derive(Debug, Clone, Copy, PartialEq)]
//     pub struct ActionClip {
//         /// The Id of the action in the [`Timeline::world`].
//         pub id: ActionId,
//         /// The offset time where the action begins.
//         pub offset: f32,
//         /// Duration of the action.
//         pub duration: f32,
//     }

//     impl ActionClip {
//         /// Get the end time of the action.
//         #[inline]
//         #[must_use]
//         pub fn end(&self) -> f32 {
//             self.offset + self.duration
//         }
//     }
// }

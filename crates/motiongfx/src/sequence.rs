use core::time::Duration;

use nonempty::NonEmpty;

use crate::action::ActionClip;

/// The [`ActionClip`]s driving one field of one subject, sorted by
/// [`ActionClip::start`].
///
/// Clips **may overlap**. Where they do, the one later in the list
/// wins. The list is sorted by start, so that is whichever began
/// most recently, not whichever was authored last. Clips it covers
/// show again when it ends, each at its own progress.
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

    #[allow(clippy::len_without_is_empty)] // It is non empty!
    #[inline]
    pub fn len(&self) -> usize {
        self.clips.len()
    }

    /// Get the start time of the sequence.
    #[inline]
    pub fn start(&self) -> Duration {
        self.clips.first().start
    }

    /// Get the end time of the sequence.
    #[inline]
    pub fn end(&self) -> Duration {
        self.clips
            .tail
            .iter()
            .fold(self.clips.head.end(), |end, clip| {
                end.max(clip.end())
            })
    }

    /// Get the duration of the sequence.
    #[inline]
    pub fn duration(&self) -> Duration {
        self.end().saturating_sub(self.start())
    }

    pub(crate) fn delay(&mut self, duration: Duration) {
        for clip in self.clips.iter_mut() {
            clip.start = clip.start.saturating_add(duration);
        }
    }

    /// Merges `other` into `self`, keeping the clips sorted by
    /// [`ActionClip::start`].
    ///
    /// Nothing is dropped; list order decides the winner, in baking
    /// and at playback alike.
    pub(crate) fn merge(&mut self, other: Self) {
        // Already sorted: append as is.
        if self.clips.last().start <= other.start() {
            self.extend(other.clips);
            return;
        }

        let NonEmpty { head, tail } = &mut self.clips;

        tail.insert(0, *head);
        tail.extend(other.clips);
        tail.sort_by_key(|clip| clip.start);
        *head = tail.remove(0);
    }
}

impl Sequence {
    /// Appends a clip.
    ///
    /// Does **not** sort. Overlapping the last clip is fine; starting
    /// before it is not.
    #[inline]
    pub fn push(&mut self, span: ActionClip) {
        debug_assert!(
            span.start >= self.clips.last().start,
            "({:?} >= {:?}) `ActionClip`s must be in start order!",
            span.start,
            self.clips.last().start,
        );

        self.clips.push(span);
    }
}

impl Extend<ActionClip> for Sequence {
    /// Appends clips without sorting, applying the same order
    /// check as [`Sequence::push`] across the batch.
    #[inline]
    fn extend<T: IntoIterator<Item = ActionClip>>(
        &mut self,
        iter: T,
    ) {
        #[cfg(debug_assertions)]
        let mut last_start = self.clips.last().start;
        #[cfg(debug_assertions)]
        let iter = {
            iter.into_iter().inspect(|clip| {
                debug_assert!(
                    clip.start >= last_start,
                    "({:?} >= {:?}) `ActionClip`s must be in \
                     start order!",
                    clip.start,
                    last_start,
                );

                last_start = clip.start;
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

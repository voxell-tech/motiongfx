use core::time::Duration;

use nonempty::NonEmpty;

use crate::action::ActionClip;

/// The [`ActionClip`]s driving one field of one subject, held in the
/// order the fragments contributing them were listed.
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
        self.clips
            .tail
            .iter()
            .fold(self.clips.head.start, |start, clip| {
                start.min(clip.start)
            })
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
}

impl Sequence {
    /// Reports every clip from `first_new` on that starts before the
    /// one ahead of it, or overlaps one earlier in the lane.
    #[cfg(feature = "tracing")]
    fn report_conflicts_from(&self, first_new: usize) {
        let mut prev_start = Duration::ZERO;
        let mut max_end = Duration::ZERO;

        for (i, clip) in self.clips.iter().enumerate() {
            if i >= first_new {
                if clip.start < prev_start {
                    tracing::error!(
                        "clip starts at {:?}, before the one ahead of it at {:?}",
                        clip.start,
                        prev_start,
                    );
                }

                // Nothing ahead reaches this clip, so nothing can
                // overlap it and the scan is skipped.
                if clip.start < max_end
                    && self.clips.iter().take(i).any(|other| {
                        clip.start < other.end()
                            && other.start < clip.end()
                    })
                {
                    tracing::error!(
                        "clip {:?}..{:?} overlaps another on the same field",
                        clip.start,
                        clip.end(),
                    );
                }
            }

            prev_start = clip.start;
            max_end = max_end.max(clip.end());
        }
    }

    /// Appends a clip.
    #[inline]
    pub fn push(&mut self, span: ActionClip) {
        self.clips.push(span);

        #[cfg(feature = "tracing")]
        self.report_conflicts_from(self.clips.len() - 1);
    }
}

impl Extend<ActionClip> for Sequence {
    /// Appends clips, preserving their order and this lane's.
    #[inline]
    fn extend<T: IntoIterator<Item = ActionClip>>(
        &mut self,
        iter: T,
    ) {
        #[cfg(feature = "tracing")]
        let first_new = self.clips.len();

        self.clips.extend(iter);

        #[cfg(feature = "tracing")]
        self.report_conflicts_from(first_new);
    }
}

impl IntoIterator for Sequence {
    type Item = ActionClip;

    type IntoIter = <NonEmpty<ActionClip> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.clips.into_iter()
    }
}

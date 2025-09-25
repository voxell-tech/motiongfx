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

    #[allow(clippy::len_without_is_empty)] // It is non empty!
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
            iter.into_iter().inspect(|clip| {
                debug_assert!(
                    clip.start >= end,
                    "({} >= {}) `ActionClip`s shouldn't overlap!",
                    clip.start,
                    end,
                );

                end = clip.end();
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

use core::time::Duration;

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

    /// Get the start time of the sequence.
    #[inline]
    pub fn start(&self) -> Duration {
        self.clips.first().start
    }

    /// Get the end time of the sequence.
    #[inline]
    pub fn end(&self) -> Duration {
        self.clips.last().end()
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
    /// clip listed ahead of it, and every earlier clip it overlaps.
    #[cfg(feature = "tracing")]
    fn report_conflicts_from(&self, first_new: usize) {
        // The latest end time among all clips before the new clips.
        let mut max_end = self
            .clips
            .iter()
            .take(first_new)
            .map(ActionClip::end)
            .max()
            .unwrap_or(Duration::ZERO);

        for (index, clip) in
            self.clips.iter().enumerate().skip(first_new)
        {
            if let Some(prev) = index
                .checked_sub(1)
                .and_then(|prev_index| self.clips.get(prev_index))
                .filter(|prev| clip.start < prev.start)
            {
                tracing::error!(
                    "`ActionClip` {} starts at {:?}, before clip {} at {:?} on the same field",
                    index,
                    clip.start,
                    index - 1,
                    prev.start,
                );
            }

            // No earlier clip reaches this clip, so nothing can
            // overlap it and the scan is skipped.
            if clip.start < max_end {
                for (before_index, before) in
                    self.clips.iter().enumerate().take(index)
                {
                    if clip.start < before.end()
                        && before.start < clip.end()
                    {
                        tracing::error!(
                            "`ActionClip` {} ({:?}..{:?}) overlaps clip {} ({:?}..{:?}) on the same field",
                            index,
                            clip.start,
                            clip.end(),
                            before_index,
                            before.start,
                            before.end(),
                        );
                    }
                }
            }

            max_end = max_end.max(clip.end());
        }
    }

    /// Appends a clip, reporting any conflict it introduces.
    #[inline]
    pub fn push(&mut self, span: ActionClip) {
        self.clips.push(span);

        #[cfg(feature = "tracing")]
        self.report_conflicts_from(self.clips.len() - 1);
    }
}

impl Extend<ActionClip> for Sequence {
    /// Appends clips, reporting any conflict they introduce.
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

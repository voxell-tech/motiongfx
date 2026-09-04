use bevy::prelude::*;
use bevy_fynix::tag::{Hovered, Pressed};
use fynix::element::element;

use super::patch::*;

/// The sized, optionally filled container almost every other widget's
/// root node turns out to be. `Host::spawn` already gives it a `Node`;
/// every field writes one slice of that, so it needs no build hook.
#[element]
pub struct Frame {
    #[elem(patch = PatchWidth)]
    pub width: Val,
    #[elem(patch = PatchHeight)]
    pub height: Val,
    /// The floor `width`/`height` can shrink to. `Auto` floors a
    /// flex item at its own content's size, which can fight a
    /// percentage `width` above trying to shrink it further; `px(0)`
    /// lifts that floor.
    #[elem(patch = PatchMinWidth)]
    pub min_width: Val,
    #[elem(patch = PatchMinHeight)]
    pub min_height: Val,
    /// Absolute for a frame that places itself, with [`inset`] saying
    /// where.
    ///
    /// [`inset`]: Frame::inset
    #[elem(patch = PatchPosition)]
    pub position: PositionType,
    /// How far each edge sits from the parent's, for an absolute
    /// frame. `Auto` on an edge leaves that one to the layout.
    #[elem(default = ::all(auto()), patch = PatchInset)]
    pub inset: UiRect,
    #[elem(patch = PatchDirection)]
    pub direction: FlexDirection,
    #[elem(patch = PatchFlexGrow)]
    pub flex_grow: f32,
    #[elem(patch = PatchFlexShrink)]
    pub flex_shrink: f32,
    #[elem(patch = PatchAlign)]
    pub align: AlignItems,
    #[elem(patch = PatchJustify)]
    pub justify: JustifyContent,
    #[elem(patch = PatchPadding)]
    pub padding: UiRect,
    #[elem(patch = PatchMargin)]
    pub margin: UiRect,
    #[elem(patch = PatchOverflow)]
    pub overflow: Overflow,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[elem(default = ::ZERO, patch = PatchRowGap)]
    pub row_gap: Val,
    #[elem(default = ::ZERO, patch = PatchColumnGap)]
    pub column_gap: Val,
    #[elem(default = ::ZERO, patch = PatchRadius)]
    pub radius: Val,
    /// Transparent by default, for a frame that only wants the
    /// layout.
    #[elem(default = ::NONE, patch = PatchBackground, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        on(Pressed, read = Self::pressed),
        on(Hovered, read = Self::hovered),
    ))]
    pub background: Color,
    /// What `background` travels to under the cursor; `None` rests.
    #[elem(ignore)]
    pub hover_background: Option<Color>,
    /// While held. Falls back to `hover_background` when unset.
    #[elem(ignore)]
    pub press_background: Option<Color>,
    /// `None` hides it, which is how a frame that depends on a size it
    /// has not measured yet avoids showing up at its intrinsic size
    /// for a frame.
    #[elem(patch = PatchDisplay)]
    pub display: Display,
    /// Where it sits in the window's stack, for a frame that has to
    /// be above what it does not sit inside. `None` leaves it in its
    /// parent's, where the tree already puts it.
    #[elem(patch = PatchOptionalZ)]
    pub z: Option<i32>,
}

impl Frame {
    /// Where `background` heads under the cursor, or its own colour
    /// when none was set, so it stays put.
    fn hovered(&self) -> &Color {
        match &self.hover_background {
            Some(color) => color,
            None => &self.background,
        }
    }

    /// While held, falling back to the hover shade.
    fn pressed(&self) -> &Color {
        match &self.press_background {
            Some(color) => color,
            None => self.hovered(),
        }
    }
}

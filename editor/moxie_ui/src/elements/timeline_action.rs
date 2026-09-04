use crate::reactive::FynixBuild;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut as _;
use bevy_fynix::tag::{Hovered, Pressed};
use fynix::element::element;

use super::Label;
use super::patch::*;

/// One action's clip on the timeline: a colored, absolutely
/// positioned, bordered hit area, its name (if any) pinned to its
/// top-left corner - clipped rather than measured, so a bar too
/// narrow for it just shows nothing instead of overflowing its
/// neighbor.
#[element(build = Self::build)]
pub struct TimelineAction {
    /// Blank when the action has no name of its own.
    #[elem(child)]
    pub label: Label,
    #[elem(patch = PatchTop)]
    pub top: Val,
    #[elem(patch = PatchLeft)]
    pub left: Val,
    #[elem(patch = PatchWidth)]
    pub width: Val,
    #[elem(patch = PatchHeight)]
    pub height: Val,
    #[elem(default = Color::NONE, patch = PatchBackground, anim(
        duration = theme.motion.interact,
        ease = theme.motion.ease,
        on(Pressed, read = Self::pressed),
        on(Hovered, read = Self::hovered),
    ))]
    pub fill: Color,
    /// What `fill` travels to under the cursor; `None` rests.
    #[elem(ignore)]
    pub hover_fill: Option<Color>,
    /// While held. Falls back to `hover_fill` when unset.
    #[elem(ignore)]
    pub press_fill: Option<Color>,
    #[elem(default = Color::NONE, patch = PatchBorderColor)]
    pub border: Color,
    /// Thickens the border - the caller still chooses `border`'s
    /// color (the theme's accent, typically).
    #[elem(patch = PatchSelected)]
    pub selected: bool,
}

impl TimelineAction {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build.insert((
            Node {
                position_type: PositionType::Absolute,
                padding: UiRect::new(
                    px(4),
                    Val::ZERO,
                    px(2),
                    Val::ZERO,
                ),
                overflow: Overflow::clip(),
                ..default()
            },
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }
}

impl TimelineAction {
    /// Where `fill` heads under the cursor, or its own colour when
    /// none was set, so it stays put.
    fn hovered(&self) -> &Color {
        match &self.hover_fill {
            Some(color) => color,
            None => &self.fill,
        }
    }

    /// While held, falling back to the hover shade.
    fn pressed(&self) -> &Color {
        match &self.press_fill {
            Some(color) => color,
            None => self.hovered(),
        }
    }
}

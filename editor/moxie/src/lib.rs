//! Timeline editor for MotionGfx, built on `bevy_ui` + `bevy_feathers`.
//!
//! Renders a docked timeline panel for the first [`Timeline`] it finds:
//! scrub by pressing/dragging the track, toggle play/pause with the
//! button or spacebar, and scroll the track (wheel/trackpad) with a
//! resizable name column.
//!
//! [`Timeline`]: bevy_motiongfx::prelude::BevyTimeline

// Inherent to Bevy ECS: systems take many params and query tuples.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod block_layout;
mod icons;
mod playback;
mod scene;
mod ui;
mod view;

use core::time::Duration;

use bevy::prelude::*;
use bevy::settings::{
    ReflectSettingsGroup, SettingsGroup, SettingsPlugin,
};
use bevy_motiongfx::prelude::TimelineId;

pub use scene::EditorScene;

/// Plugin that renders a timeline editor UI for the first
/// [`Timeline`](bevy_motiongfx::prelude::BevyTimeline).
pub struct MoxiePlugin;

impl Plugin for MoxiePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SettingsPlugin::new(
            "org.voxell.motiongfx.editor",
        ))
        .add_plugins(ui::UiPlugin);
    }
}

/// Pixels per second of animation (horizontal zoom).
pub(crate) const PIXELS_PER_SECOND: f32 = 160.0;

/// Horizontal pixel offset for a point `t` into the timeline.
#[inline]
pub(crate) fn px_for(t: Duration) -> f32 {
    t.as_secs_f32() * PIXELS_PER_SECOND
}

/// The offscreen texture the composition's scene cameras render into.
/// `bevy_ui` scales this image to fit the preview area above the
/// timeline panel, so growing the panel shrinks the whole frame
/// uniformly instead of distorting it. Sized from
/// [`EditorSettings::physical_size`].
#[derive(Resource)]
pub(crate) struct PreviewImage(pub(crate) Handle<Image>);

/// The focused timeline and its duration.
#[derive(Resource, Default)]
pub(crate) struct EditorState {
    pub(crate) timeline: Option<TimelineId>,
    pub(crate) duration: Duration,
    /// Mirrored from the first [`RealtimePlayer`](bevy_motiongfx::prelude::RealtimePlayer)
    /// so the play/pause label can bind to this resource instead of
    /// polling a component query. Written by `on_toggle_playback` and
    /// `stop_at_track_end`.
    pub(crate) is_playing: bool,
}

#[derive(Debug, Resource, SettingsGroup, Reflect)]
#[reflect(Resource, SettingsGroup, Default)]
pub struct EditorSettings {
    hdr: bool,
    physical_size: UVec2,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            hdr: Default::default(),
            // Portrait 9:16 to match the current compositions; the
            // offscreen preview renders at this resolution.
            physical_size: UVec2::new(1080, 1920),
        }
    }
}

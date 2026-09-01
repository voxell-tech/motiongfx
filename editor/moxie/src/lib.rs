//! Timeline editor for MotionGfx, built on `bevy_ui` + `bevy_feathers`.
//!
//! Renders a docked timeline panel for the first [`Timeline`] it finds:
//! scrub by pressing/dragging the track, toggle play/pause with the
//! button or spacebar, and scroll the track (wheel/trackpad) with a
//! resizable name column.
//!
//! [`Timeline`]: bevy_motiongfx::prelude::BevyTimeline

#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Inherent to Bevy ECS: systems take many params and query tuples."
)]

mod block_layout;
mod icons;
mod playback;
mod project;
mod scene;
mod time_axis;
mod ui;
mod view;
mod zoom;

use core::time::Duration;
use std::path::PathBuf;

use bevy::prelude::*;
use bevy::settings::{
    ReflectSettingsGroup, SettingsGroup, SettingsPlugin,
};
use bevy_motiongfx::prelude::TimelineId;
use bevy_motiongfx::scene::id::EntityUid;

use moxie_asset::MoxieAssetPlugin;
pub use scene::EditorScene;

/// Plugin that renders a timeline editor UI for the first
/// [`Timeline`](bevy_motiongfx::prelude::BevyTimeline).
pub struct MoxiePlugin;

impl Plugin for MoxiePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SettingsPlugin::new("org.voxell.motiongfx.editor"),
            MoxieAssetPlugin,
            ui::UiPlugin,
        ))
        .add_systems(PreUpdate, ensure_scene_root);
    }
}

/// Ensures an [`Entity`] with [`SceneRoot`] exists.
pub(crate) fn ensure_scene_root(
    mut commands: Commands,
    roots: Query<Entity, With<SceneRoot>>,
    root_subjects: Query<Entity, (With<EntityUid>, Without<ChildOf>)>,
) {
    let root_count = roots.count();
    if root_count > 1 {
        error!("There are more than one root in the scene!");
    } else if root_count == 0 {
        let root = commands
            .spawn((
                SceneRoot,
                Transform::IDENTITY,
                Visibility::Inherited,
            ))
            .id();

        for subject in &root_subjects {
            commands.entity(root).add_child(subject);
        }
    }
}

/// Marker component for the root [`Entity`] of the scene.
/// All subjects with [`EntityUid`] lives under this.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Default, Clone)]
pub struct SceneRoot;

/// Zoom range, spanning the scales the time axis is exercised at.
const MIN_PX_PER_SECOND: f32 = 1.0;
const MAX_PX_PER_SECOND: f32 = 20_000.0;

/// Maps animation time to timeline pixels.
#[derive(Resource, Clone, Copy, PartialEq)]
pub(crate) struct TimelineView {
    px_per_second: f32,
    /// Where the timeline's left edge sits.
    offset: Duration,
}

impl Default for TimelineView {
    fn default() -> Self {
        Self {
            px_per_second: 160.0,
            offset: Duration::ZERO,
        }
    }
}

impl TimelineView {
    /// Horizontal pixel offset for a point `t` into the timeline.
    #[inline]
    pub(crate) fn x_from_time(&self, t: Duration) -> f32 {
        let secs = if t >= self.offset {
            (t - self.offset).as_secs_f32()
        } else {
            -(self.offset - t).as_secs_f32()
        };
        secs * self.px_per_second
    }

    /// Point into the timeline at `x`, clamped to a non-negative
    /// time.
    #[inline]
    pub(crate) fn time_from_x(&self, x: f32) -> Duration {
        let secs = x / self.px_per_second;
        if !secs.is_finite() {
            return self.offset;
        }
        let step = Duration::from_secs_f32(secs.abs());
        if secs >= 0.0 {
            self.offset.saturating_add(step)
        } else {
            self.offset.saturating_sub(step)
        }
    }

    /// Scale the zoom by `factor` and leave `anchor_time` sitting at
    /// `anchor_x`, saturating at the ends of the range.
    pub(crate) fn zoom_to(
        &mut self,
        anchor_x: f32,
        anchor_time: Duration,
        factor: f32,
    ) {
        if !(factor.is_finite() && factor > 0.0) {
            return;
        }
        self.px_per_second = (self.px_per_second * factor)
            .clamp(MIN_PX_PER_SECOND, MAX_PX_PER_SECOND);
        // Put the anchor at the left edge, then push it back to
        // `anchor_x`.
        self.offset = anchor_time;
        self.pan_by(anchor_x);
    }

    /// Slide the view `delta_x` pixels along the timeline, stopping at
    /// the start.
    pub(crate) fn pan_by(&mut self, delta_x: f32) {
        self.offset = self.time_from_x(-delta_x);
    }

    /// Scale the view so a `duration` long animation spans a `width`
    /// px panel, leaving a little room after it.
    pub(crate) fn fit(&mut self, width: f32, duration: Duration) {
        let secs = duration.as_secs_f32();
        if secs <= 0.0 {
            return;
        }
        self.px_per_second = (width / (secs * 1.02))
            .clamp(MIN_PX_PER_SECOND, MAX_PX_PER_SECOND);
        self.offset = Duration::ZERO;
    }
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

/// The path (root-to-node child indices) of the action currently
/// selected in the timeline panel, if any. `None` selects nothing.
#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct SelectedAction(pub(crate) Option<Vec<usize>>);

/// The entity currently selected in the hierarchy panel, if any.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) struct SelectedEntity(pub(crate) Option<Entity>);

/// Folders bookmarked for browsing in the asset panel. Saved and
/// loaded with the project: a bookmark only means something alongside
/// the assets it points at.
#[derive(Resource, Default, Clone)]
pub(crate) struct ProjectBookmarks(pub(crate) Vec<PathBuf>);

/// Where the open project's own `.mox` was last loaded from or saved
/// to. Its folder is the asset panel's own, permanent bookmark.
#[derive(Resource, Default, Clone)]
pub(crate) struct ProjectPath(pub(crate) Option<PathBuf>);

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
            physical_size: UVec2::new(1920, 1080),
        }
    }
}

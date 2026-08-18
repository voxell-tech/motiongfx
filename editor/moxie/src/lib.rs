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
mod ui;
mod view;

use core::time::Duration;

use bevy::prelude::*;
use bevy::settings::{
    ReflectSettingsGroup, SettingsGroup, SettingsPlugin,
};
use bevy_motiongfx::prelude::TimelineId;
use bevy_motiongfx::scene::id::EntityUid;

pub use scene::EditorScene;

/// Plugin that renders a timeline editor UI for the first
/// [`Timeline`](bevy_motiongfx::prelude::BevyTimeline).
pub struct MoxiePlugin;

impl Plugin for MoxiePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SettingsPlugin::new(
            "org.voxell.motiongfx.editor",
        ))
        .add_plugins(ui::UiPlugin)
        // What a saved subject is made of, beyond what the inspector
        // and `bevy_motiongfx` already register between them. Nothing
        // reaches an unregistered type through reflection, so a
        // project file would quietly lose these.
        .register_type::<Visibility>()
        .register_type::<ChildOf>()
        .register_type::<Mesh3d>()
        .register_type::<SceneRoot>();
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

/// What every subject hangs under, so that [`Children`] is where a
/// subject's place in the scene lives, at every depth. Without it the
/// top level would have no order to speak of: nothing relates one
/// parentless entity to the next.
///
/// Deliberately not a subject itself. With no
/// [`EntityUid`](bevy_motiongfx::scene::id::EntityUid) the animation
/// cannot address it and the hierarchy panel never draws it. It is
/// saved regardless, because a child's [`ChildOf`] has to resolve to
/// something when the project is read back.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct SceneRoot;

/// Keeps the scene rooted: spawns a [`SceneRoot`] when there is none,
/// and takes in any subject left outside it.
///
/// Adoption is what lets a subject be spawned without knowing the root
/// exists. `main.rs` builds its scene top-level, and a project written
/// before the root did comes back the same way; both are taken in on
/// the frame they arrive. The root sits at the identity transform, so
/// nothing moves by being adopted.
pub(crate) fn ensure_scene_root(
    mut commands: Commands,
    roots: Query<Entity, With<SceneRoot>>,
    subjects: Query<(Entity, Option<&ChildOf>), With<EntityUid>>,
    in_scene: Query<(), Or<(With<EntityUid>, With<SceneRoot>)>>,
) {
    let root = match roots.iter().next() {
        Some(root) => root,
        None => commands
            .spawn((
                SceneRoot,
                Transform::default(),
                Visibility::default(),
            ))
            .id(),
    };

    for (subject, parent) in &subjects {
        // A parent that is gone reads the same as none: a project can
        // name one that was never saved.
        let held = parent
            .is_some_and(|parent| in_scene.contains(parent.parent()));

        if !held {
            commands.entity(root).add_child(subject);
        }
    }
}

/// The path (root-to-node child indices) of the action currently
/// selected in the timeline panel, if any. `None` selects nothing.
#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct SelectedAction(pub(crate) Option<Vec<usize>>);

/// The entity currently selected in the hierarchy panel, if any.
#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub(crate) struct SelectedEntity(pub(crate) Option<Entity>);

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

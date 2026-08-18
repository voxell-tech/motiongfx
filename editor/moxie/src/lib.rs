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
pub mod std_material_asset;
mod ui;
mod view;

use core::time::Duration;

use bevy::ecs::reflect::AppTypeRegistry;
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
        .add_systems(PreUpdate, ensure_scene_root);

        let registry =
            app.world().resource::<AppTypeRegistry>().clone();
        app.register_asset_loader(
            std_material_asset::MaterialAssetLoader::new(&registry),
        );
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

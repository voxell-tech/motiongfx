//! A timeline editor for MotionGfx, built on `bevy_ui` and styled
//! with `bevy_feathers` on top of the headless `bevy_ui_widgets`
//! behaviors.
//!
//! Renders a bottom-docked timeline panel focused on the first track
//! of the first [`Timeline`] it finds:
//! - every action is a clip box, positioned by its start/duration and
//!   colored by its subject entity;
//! - concurrent groups (all / any / flow) get a collapsible
//!   container;
//! - pressing or dragging anywhere on the track scrubs, mapping the
//!   cursor onto the timeline via `PIXELS_PER_SECOND`;
//! - a feathers [`Button`] (or the spacebar) toggles play/pause;
//! - the track scrolls (wheel / trackpad) via a [`ScrollArea`], with
//!   a resizable name column.
//!
//! Modules: [`scene`] (component markers + `bsn!` tree + setup),
//! [`layout`] (composition tree → clip/group/toggle placements),
//! [`playback`] (play/pause, scrub, playhead), [`view`] (camera +
//! scroll sync). Widgets live in `motiongfx_editor_ui`.
//!
//! [`Timeline`]: bevy_motiongfx::prelude::BevyTimeline
//! [`Button`]: bevy::ui_widgets::Button
//! [`ScrollArea`]: bevy::ui_widgets::ScrollArea

// Inherent to Bevy ECS: systems take many params and query tuples.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod hierarchy;
mod layout;
mod playback;
mod scene;
mod view;

use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::settings::{
    ReflectSettingsGroup, SettingsGroup, SettingsPlugin,
};
use bevy_motiongfx::prelude::TimelineId;
use motiongfx_editor_ui::dock::DockPlugin;
use motiongfx_editor_ui::inspector::ReflectInspectorPlugin;

/// Plugin that renders a timeline editor UI for the first
/// [`Timeline`](bevy_motiongfx::prelude::BevyTimeline).
pub struct MotionGfxEditorPlugin;

impl Plugin for MotionGfxEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(SettingsPlugin::new(
            "org.voxell.motiongfx.editor",
        ))
        .add_plugins(EditorUiPlugin);
    }
}

/// Wires feathers theming, the editor scene, and the per-frame
/// timeline/playback/preview systems.
struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FeathersPlugins,
            DockPlugin,
            ReflectInspectorPlugin::<EditorSettings>::default(),
        ))
            // Seed the feathers palette (its default theme is empty).
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<EditorState>()
            .init_resource::<hierarchy::HierarchyState>()
            .add_systems(Startup, scene::setup_editor_ui)
            .add_systems(
                Update,
                (
                    layout::build_timeline_view,
                    hierarchy::build_hierarchy_view,
                    playback::play_pause_hotkey,
                    playback::update_playhead,
                    playback::stop_at_track_end,
                    playback::update_play_label,
                    view::sync_name_scroll,
                    view::retarget_scene_cameras,
                    view::fit_preview_image,
                )
                    .chain(),
            )
            .add_observer(playback::on_toggle_playback)
            .add_observer(layout::on_timeline_content_added);
    }
}

/// Pixels per second of animation (horizontal zoom).
pub(crate) const PIXELS_PER_SECOND: f32 = 160.0;
pub(crate) const PANEL_PADDING: f32 = 12.0;
pub(crate) const NAME_PANEL_WIDTH: f32 = 140.0;
pub(crate) const NAME_PANEL_MIN: f32 = 60.0;
pub(crate) const NAME_PANEL_MAX: f32 = 400.0;
pub(crate) const CONTROL_BAR_HEIGHT: f32 = 40.0;
pub(crate) const ROW_HEIGHT: f32 = 22.0;
pub(crate) const ROW_STRIDE: f32 = ROW_HEIGHT + 8.0;
pub(crate) const TRACK_TOP_PADDING: f32 = 12.0;

/// The offscreen texture the composition's scene cameras render into.
/// `bevy_ui` scales this image to fit the preview area above the
/// timeline panel, so growing the panel shrinks the whole frame
/// uniformly instead of distorting it. Sized from
/// [`EditorSettings::physical_size`].
#[derive(Resource)]
pub(crate) struct PreviewImage(pub(crate) Handle<Image>);

/// Shared editor state: the focused timeline, whether the view is up
/// to date, and which groups are collapsed.
#[derive(Resource, Default)]
pub(crate) struct EditorState {
    pub(crate) timeline: Option<TimelineId>,
    /// Whether the timeline view is up to date. Cleared to force a
    /// rebuild (e.g. after a group is collapsed/expanded).
    pub(crate) built: bool,
    pub(crate) duration: f32,
    /// Ids of the concurrent groups currently collapsed.
    pub(crate) collapsed: HashSet<usize>,
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

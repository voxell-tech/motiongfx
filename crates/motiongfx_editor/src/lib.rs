//! A timeline editor for MotionGfx, built on `bevy_ui` and styled with
//! `bevy_feathers` on top of the headless `bevy_ui_widgets` behaviors.
//!
//! Renders a bottom-docked timeline panel focused on the first track of
//! the first [`Timeline`] it finds:
//! - every action is a rounded box, grouped into rows by field;
//! - the whole track is a [`Slider`] acting as the scrubber, with the
//!   playhead as its thumb;
//! - a feathers [`Button`] toggles play/pause;
//! - the track scrolls (wheel / trackpad) via a [`ScrollArea`] when it
//!   is wider or taller than the panel, with a resizable name column.
//!
//! [`Timeline`]: bevy_motiongfx::prelude::BevyTimeline
//! [`Slider`]: bevy::ui_widgets::Slider
//! [`Button`]: bevy::ui_widgets::Button
//! [`ScrollArea`]: bevy::ui_widgets::ScrollArea

mod ui;

use core::time::Duration;

use bevy::camera::{ClearColorConfig, RenderTarget, Viewport};
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::{ThemeBackgroundColor, UiTheme};
use bevy::feathers::{FeathersPlugins, tokens};
use bevy::prelude::*;
use bevy::ui::{ScrollPosition, UiTargetCamera};
use bevy::ui_widgets::{
    Activate, ControlOrientation, ScrollArea, SliderRange,
    SliderValue, ValueChange,
};
use bevy::window::PrimaryWindow;
use bevy_motiongfx::motiongfx::action::ActionKey;
use bevy_motiongfx::motiongfx::track::Span;
use bevy_motiongfx::prelude::*;

use ui::{
    Divider, action_box, label, on_divider_drag, on_panel_resize,
    playhead_line, row_color, row_label, scrub_slider, themed_button,
};

/// Pixels per second of animation (horizontal zoom).
const PIXELS_PER_SECOND: f32 = 160.0;
const PANEL_HEIGHT: f32 = 180.0;
const PANEL_MIN_HEIGHT: f32 = 100.0;
const PANEL_MAX_HEIGHT: f32 = 900.0;
const PANEL_PADDING: f32 = 12.0;
const NAME_PANEL_WIDTH: f32 = 140.0;
const NAME_PANEL_MIN: f32 = 60.0;
const NAME_PANEL_MAX: f32 = 400.0;
const CONTROL_BAR_HEIGHT: f32 = 40.0;
const ROW_HEIGHT: f32 = 32.0;
const ROW_STRIDE: f32 = ROW_HEIGHT + 8.0;
const TRACK_TOP_PADDING: f32 = 12.0;

/// Plugin that renders a timeline editor UI for the first
/// [`Timeline`](bevy_motiongfx::prelude::BevyTimeline).
pub struct MotionGfxEditorPlugin;

impl Plugin for MotionGfxEditorPlugin {
    fn build(&self, app: &mut App) {
        // The headless `bevy_ui_widgets` behaviors (Button, Slider,
        // ScrollArea, ...) are registered by `DefaultPlugins` when the
        // `bevy_ui_widgets` feature is on; we only add the feathers
        // theme layer on top.
        app.add_plugins(FeathersPlugins)
            // Seed the feathers palette (its default theme is empty).
            .insert_resource(UiTheme(create_dark_theme()))
            .init_resource::<EditorState>()
            .add_systems(Startup, setup_editor_ui)
            .add_systems(
                Update,
                (
                    build_timeline_view,
                    update_playhead,
                    stop_at_track_end,
                    update_play_label,
                    sync_name_scroll,
                    update_camera_viewport,
                )
                    .chain(),
            )
            .add_observer(on_play_pause);
    }
}

#[derive(Resource, Default)]
struct EditorState {
    timeline: Option<TimelineId>,
    built: bool,
    duration: f32,
}

/// Marker for the root panel node (its height is resizable).
#[derive(SceneComponent, Default, Clone)]
pub struct EditorPanel;

impl EditorPanel {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(PANEL_HEIGHT),
                flex_direction: FlexDirection::Column,
                padding: UiRect::bottom(Val::Px(PANEL_PADDING)),
            }
            EditorPanel
            ThemeBackgroundColor(tokens::WINDOW_BG)
            Children [
            // --- Top-edge grab handle to resize the panel height. ---
                (
                    @Divider
                    on(on_panel_resize)
                ),
            // --- Control bar: play/pause + time readout. ---
                (
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(CONTROL_BAR_HEIGHT),
                        flex_shrink: 0.0,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(12.0),
                        padding: UiRect::horizontal(Val::Px(
                            PANEL_PADDING,
                        )),
                    }
                    Children [
                        (
                            themed_button::<PlayPauseButton>(
                                84.0,
                                26.0,
                            )
                            Children [
                                label::<PlayPauseLabel>("Play")
                            ]
                        ),
                        (
                            label::<TimeLabel>("0.00s")
                        ),
                    ]
                ),

            // --- Track area: name column | divider | scroll viewport. ---

                (
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        // Allow this flex item to shrink below its content
                        // height so the viewport below can clip and scroll
                        // (flex items default to `min-height: auto`).
                        min_height: Val::Px(0.0),
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::horizontal(Val::Px(
                            PANEL_PADDING,
                        )),
                    }

                    Children [
                        (
                            NamePanel
                            Node {
                                width: Val::Px(NAME_PANEL_WIDTH),
                                height: Val::Percent(100.0),
                                min_height: Val::Px(0.0),
                                flex_shrink: 0.0,
                                flex_direction: FlexDirection::Column,
                                overflow: Overflow::scroll_y(),
                                padding: UiRect::top(Val::Px(
                                    TRACK_TOP_PADDING,
                                )),
                            }
                            ThemeBackgroundColor(tokens::PANE_BODY_BG)
                        ),
                        (
                            @Divider {
                                @thickness: Val::Px(4.0),
                                @orientation: ControlOrientation::Vertical
                            }
                            on(on_divider_drag)
                        ),
                        (
                            @TrackViewport
                        ),
                    ]
                )
            ]
        }
    }
}

/// Marks camera which renders to the editor panel
/// Other camera's are kept
/// above the editor panel (aspect ratio is preserved automatically).
#[derive(Component, Default, Clone)]
pub struct TrackViewportCamera;

/// Viewport where the Timeline, Track and action UI is displayed
#[derive(SceneComponent, Default, Clone)]
struct TrackViewport;

impl TrackViewport {
    fn scene() -> impl Scene {
        bsn! {
                TrackViewport
                ScrollArea
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    // `min: 0` lets the viewport shrink
                    // below its (tall/wide) content so it
                    // actually clips and scrolls.
                    min_width: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    overflow: Overflow::scroll(),
                }
                ThemeBackgroundColor(
                    tokens::PANE_BODY_BG,
                )

                Children [
                    TimelineContent
                    scrub_slider(1.0, 1.0)
                    on(on_scrub)
                    Children [
                        Playhead
                        playhead_line(0.0)
                    ]
                ]

        }
    }
}

/// The scrubbable track: a horizontal slider whose value is playback
/// time in seconds. Holds the action boxes and the playhead thumb.
///
/// The static skeleton is spawned by [`TrackViewport`]; its size, time
/// range and action boxes are filled in by [`build_timeline_view`] once
/// a timeline exists.
#[derive(Component, Default, Clone)]
struct TimelineContent;

#[derive(Component, Default, Clone)]
pub struct NamePanel;

#[derive(Component, Default, Clone)]
struct Playhead;

#[derive(Component, Default, Clone)]
struct PlayPauseButton;

#[derive(Component, Default, Clone)]
struct PlayPauseLabel;

#[derive(Component, Default, Clone)]
struct TimeLabel;

fn setup_editor_ui(mut commands: Commands) {
    // The editor UI renders to its own full-window 2D camera so that
    // confining a scene camera's viewport (see `update_camera_viewport`)
    // doesn't also shrink the UI. `ClearColorConfig::None` keeps the
    // scene camera's output; `order: 1` draws the UI on top.

    let ui_camera = commands
        .spawn_scene(bsn! [
            Camera2d
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
            }
            TrackViewportCamera
        ])
        .id();
    commands.spawn(UiTargetCamera(ui_camera)).apply_scene(bsn! {
        @EditorPanel
    });
}

/// Fills in the timeline view once a timeline exists. Runs every frame
/// until it finds one (the timeline is built in the user's `Startup`),
/// then latches via [`EditorState::built`].
///
/// Only the first track is shown.
fn build_timeline_view(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    manager: Res<MotionGfxManager>,
    q_timelines: Query<&TimelineId>,
    mut q_nodes: Query<&mut Node, Without<Playhead>>,
    q_content: Query<Entity, With<TimelineContent>>,
    q_name_panel: Query<Entity, With<NamePanel>>,
) {
    if state.built {
        return;
    }

    let Some(timeline_id) = q_timelines.iter().next().copied() else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    // Focus on the first track only.
    let Some(track) = timeline.tracks().first() else {
        return;
    };
    let Ok(content) = q_content.single() else {
        return;
    };
    let Ok(name_panel) = q_name_panel.single() else {
        return;
    };

    // Order rows by first-clip start so the layout reads left-to-right.
    let mut rows: Vec<(&ActionKey, &Span)> = track
        .sequences_spans()
        .iter()
        .map(|(k, s)| (k, s))
        .collect();
    rows.sort_by_key(|(_, span)| {
        track
            .clips(**span)
            .first()
            .map(|c| c.start)
            .unwrap_or(Duration::MAX)
    });

    // The layout below is pixel math, so drop to seconds here.
    let duration = track.duration().as_secs_f32();
    let content_width = (duration * PIXELS_PER_SECOND).max(1.0);
    let content_height =
        TRACK_TOP_PADDING * 2.0 + rows.len() as f32 * ROW_STRIDE;

    // Size the slider track and give it the real time range.
    if let Ok(mut node) = q_nodes.get_mut(content) {
        node.width = Val::Px(content_width);
        node.min_width = Val::Px(content_width);
        node.height = Val::Px(content_height);
        node.min_height = Val::Px(content_height);
    }
    commands
        .entity(content)
        .insert(SliderRange::new(0.0, duration.max(f32::EPSILON)));

    // Spawn each row: a name-column label plus one box per clip.
    // `ChildOf` appends, leaving the playhead thumb child intact.
    for (row, (key, span)) in rows.iter().enumerate() {
        let color = row_color(row);
        let top = TRACK_TOP_PADDING + row as f32 * ROW_STRIDE;

        commands
            .spawn_scene(row_label(key.field().field_path()))
            .insert(ChildOf(name_panel));

        for clip in track.clips(**span) {
            let left = clip.start.as_secs_f32() * PIXELS_PER_SECOND;
            let width = (clip.duration.as_secs_f32()
                * PIXELS_PER_SECOND)
                .max(2.0);

            commands
                .spawn_scene(action_box(
                    left, top, width, ROW_HEIGHT, color,
                ))
                .insert(ChildOf(content));
        }
    }

    state.timeline = Some(timeline_id);
    state.duration = duration;
    state.built = true;
}

/// Scrub the timeline in response to slider drags.
fn on_scrub(
    change: On<ValueChange<f32>>,
    state: Res<EditorState>,
    mut commands: Commands,
    mut manager: ResMut<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };

    // Stop playback so the scrub isn't immediately overwritten.
    for mut player in &mut q_players {
        player.is_playing = false;
    }

    // Write the value back (`SliderValue` is a controlled component):
    // this keeps the slider's drag offset correct for absolute
    // cursor-following.
    commands
        .entity(change.source)
        .insert(SliderValue(change.value));

    if let Some(timeline) = manager.get_timeline_mut(&timeline_id) {
        timeline.set_target_track(0);
        timeline.set_target_time(change.value);
    }
}

/// Toggle playback when the play/pause button is activated.
fn on_play_pause(
    activate: On<Activate>,
    state: Res<EditorState>,
    mut manager: ResMut<MotionGfxManager>,
    q_button: Query<(), With<PlayPauseButton>>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    if q_button.get(activate.entity).is_err() {
        return;
    }

    for mut player in &mut q_players {
        player.is_playing = !player.is_playing;
        player.time_scale = 1.0;
    }

    // Rewind if starting playback from the very end.
    if let Some(timeline_id) = state.timeline
        && q_players.iter().any(|p| p.is_playing)
        && let Some(timeline) = manager.get_timeline_mut(&timeline_id)
        && timeline.target_time().as_secs_f32() >= state.duration
    {
        timeline.set_target_track(0);
        timeline.set_target_time(0.0);
    }
}

/// Move the playhead / slider thumb to the current target time and
/// update the time readout.
fn update_playhead(
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut commands: Commands,
    mut q_playhead: Query<&mut Node, With<Playhead>>,
    q_value: Query<(Entity, &SliderValue), With<TimelineContent>>,
    mut q_time_label: Query<&mut Text, With<TimeLabel>>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    let time = timeline.target_time().as_secs_f32();

    for mut node in &mut q_playhead {
        node.left = Val::Px(time * PIXELS_PER_SECOND);
    }
    // Keep the controlled slider value tracking playback so a grab
    // mid-playback starts its drag from the right offset.
    if let Ok((content, value)) = q_value.single()
        && value.0 != time
    {
        commands.entity(content).insert(SliderValue(time));
    }
    for mut text in &mut q_time_label {
        *text = Text::new(format!("{time:.2}s"));
    }
}

/// Clear [`RealtimePlayer::is_playing`] once playback reaches the end
/// of the current track.
///
/// [`Timeline::set_target_time`] clamps to the track's duration, so the
/// player would otherwise keep "playing" against the clamp and the
/// button would stay stuck on "Pause".
///
/// [`Timeline::set_target_time`]: bevy_motiongfx::prelude::Timeline::set_target_time
fn stop_at_track_end(
    state: Res<EditorState>,
    manager: Res<MotionGfxManager>,
    mut q_players: Query<&mut RealtimePlayer>,
) {
    let Some(timeline_id) = state.timeline else {
        return;
    };
    let Some(timeline) = manager.get_timeline(&timeline_id) else {
        return;
    };
    if state.duration <= 0.0 {
        return;
    }

    // Playing backwards stops at the start instead.
    for mut player in &mut q_players {
        if !player.is_playing {
            continue;
        }
        let at_end = if player.time_scale >= 0.0 {
            timeline.target_time().as_secs_f32() >= state.duration
        } else {
            timeline.target_time() == Duration::ZERO
        };
        if at_end {
            player.is_playing = false;
        }
    }
}

fn update_play_label(
    q_players: Query<&RealtimePlayer>,
    mut q_label: Query<&mut Text, With<PlayPauseLabel>>,
) {
    let Some(player) = q_players.iter().next() else {
        return;
    };
    let Ok(mut text) = q_label.single_mut() else {
        return;
    };
    let want = if player.is_playing { "Pause" } else { "Play" };
    if text.as_str() != want {
        *text = Text::new(want);
    }
}

/// Confine every window-targeting scene camera (i.e. all but the
/// editor's own [`TrackViewportCamera`]) to the region above the panel.
/// The camera system re-derives the projection's aspect ratio from this
/// viewport, so nothing is stretched.
fn update_camera_viewport(
    q_panel: Query<&ComputedNode, With<EditorPanel>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut q_camera: Query<
        (&mut Camera, Option<&RenderTarget>),
        Without<TrackViewportCamera>,
    >,
) {
    let Ok(panel) = q_panel.single() else {
        return;
    };
    let Ok(window) = q_window.single() else {
        return;
    };

    // `ComputedNode::size` and `Window::physical_*` are both in
    // physical pixels, which is what `Viewport` expects.
    let win =
        UVec2::new(window.physical_width(), window.physical_height());
    let panel_h = panel.size().y.round() as u32;
    let size = UVec2::new(
        win.x.max(1),
        win.y.saturating_sub(panel_h).max(1),
    );

    for (mut camera, target) in &mut q_camera {
        // Cameras rendering to a texture aren't sized by the window.
        // An absent `RenderTarget` defaults to the primary window.
        if target
            .is_some_and(|t| !matches!(t, RenderTarget::Window(_)))
        {
            continue;
        }

        let unchanged = camera.viewport.as_ref().is_some_and(|v| {
            v.physical_position == UVec2::ZERO
                && v.physical_size == size
        });
        if !unchanged {
            camera.viewport = Some(Viewport {
                physical_position: UVec2::ZERO,
                physical_size: size,
                ..default()
            });
        }
    }
}

/// Keep the name column's vertical scroll locked to the track viewport.
fn sync_name_scroll(
    q_viewport: Query<&ScrollPosition, With<TrackViewport>>,
    mut q_name_panel: Query<
        &mut ScrollPosition,
        (With<NamePanel>, Without<TrackViewport>),
    >,
) {
    let Ok(viewport) = q_viewport.single() else {
        return;
    };
    let Ok(mut name_scroll) = q_name_panel.single_mut() else {
        return;
    };
    if name_scroll.y != viewport.y {
        name_scroll.y = viewport.y;
    }
}

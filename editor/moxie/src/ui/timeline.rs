//! The timeline panel: control bar (play/pause + time readout), name
//! column, divider and scrubbable track viewport.

use core::time::Duration;

use bevy::picking::events::{Click, Drag, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::{ControlOrientation, ScrollArea};
use bevy_motiongfx::prelude::MotionGfxManager;

use super::PANEL_PADDING;
use crate::EditorState;
use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_click_release,
    on_track_drag, on_track_press, on_track_release,
};
use bevy_fynix::ElementMutExt;
use fynix_mock::elem;
use moxie_ui::fynix::{
    Button, ButtonCursor, ButtonLook, Divider, Frame, Icon,
    IconCursor, Label, LabelCursor, Panel, PanelCursor, PlayheadLine,
    PlayheadLineCursor, TimelineTrack, TimelineTrackCursor,
};
use moxie_ui::reactive::{BevyUi, resource_changed, value_changed};
use moxie_ui::theme::EditorTheme;

const NAME_PANEL_WIDTH: f32 = 140.0;
const NAME_PANEL_MIN: f32 = 60.0;
const NAME_PANEL_MAX: f32 = 400.0;
const CONTROL_BAR_HEIGHT: f32 = 40.0;
const TRACK_TOP_PADDING: f32 = 12.0;
/// Height of one track's box, and the gap below it.
const TRACK_HEIGHT: f32 = 22.0;
const TRACK_GAP: f32 = 4.0;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(Component, Default, Clone)]
struct TrackViewport;

/// The scrubbable track, sized to the timeline's duration at
/// [`PIXELS_PER_SECOND`](crate::PIXELS_PER_SECOND). Holds the track
/// boxes and the playhead; scrubbing comes from pointer observers on
/// it, so a drag can only start from a press that lands inside.
#[derive(Component, Default, Clone)]
pub(crate) struct TimelineContent;

#[derive(Component, Default, Clone)]
struct NamePanel;

/// The timeline panel, as kernel nodes.
///
/// Each reactive field binds at the node that owns it, which is why
/// this is a builder rather than a `bsn!` tree: the play/pause icon,
/// time label and friends have to be `NodeMut`s to carry their own
/// binds.
pub(super) fn panel(ui: &mut BevyUi) {
    ui.elem(elem!(!Panel {
        direction = FlexDirection::Column;
        padding = UiRect::bottom(Val::Px(PANEL_PADDING))
    }))
    .with(|ui| {
        control_bar(ui);
        track_area(ui);
    });
}

/// Play/pause + time readout.
fn control_bar(ui: &mut BevyUi) {
    ui.elem(elem!(!Frame {
        width = Val::Percent(100.0);
        height = Val::Px(CONTROL_BAR_HEIGHT);
        align = AlignItems::Center;
        gap = Val::Px(12.0);
        padding = UiRect::horizontal(Val::Px(PANEL_PADDING))
    }))
    .with(|ui| {
        ui.elem(elem!(!Button {
            look = ButtonLook::Normal;
            width = Val::Px(26.0);
            height = Val::Px(26.0);
            radius = Val::Px(6.0);
            icon = Icon {
                image: crate::icons::PLAY.into(),
                size: Val::Px(14.0),
                ..default()
            }
        }))
        .observe(
            |mut click: On<Pointer<Click>>,
             mut commands: Commands| {
                click.propagate(false);
                commands.trigger(TogglePlayback);
            },
        )
        .bind(
            |button| button.icon().image(),
            resource_changed::<EditorState>(),
            |world, _| {
                if world.resource::<EditorState>().is_playing {
                    crate::icons::PAUSE.to_string()
                } else {
                    crate::icons::PLAY.to_string()
                }
            },
        );

        ui.elem(elem!(!Label { text = "0.00s" })).bind(
            |label| label.text(),
            resource_changed::<MotionGfxManager>(),
            |world, entity| {
                format!(
                    "{:.2}s",
                    current_time(world, entity).as_secs_f32()
                )
            },
        );
    });
}

/// Name column | divider | scroll viewport.
fn track_area(ui: &mut BevyUi) {
    ui.elem(elem!(!Panel {
        direction = FlexDirection::Row;
        padding = UiRect::horizontal(Val::Px(PANEL_PADDING))
    }))
    .with(|ui| {
        ui.elem(elem!(!Panel {
            direction = FlexDirection::Column;
            padding = UiRect::top(Val::Px(TRACK_TOP_PADDING));
            scrolls = true
        }))
        .insert((
            NamePanel,
            Node {
                width: Val::Px(NAME_PANEL_WIDTH),
                height: Val::Percent(100.0),
                min_height: Val::Px(0.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::top(Val::Px(TRACK_TOP_PADDING)),
                ..default()
            },
        ))
        // Locked to the track viewport, which is found as a sibling:
        // the builder cannot know its entity yet.
        .bind(
            |panel| panel.scroll(),
            value_changed(viewport_scroll),
            viewport_scroll,
        );

        ui.elem(elem!(!Divider {
            thickness = Val::Px(4.0);
            orientation = ControlOrientation::Vertical
        }))
        .observe(on_divider_drag);

        ui.elem(elem!(!Frame { width = Val::Percent(100.0) }))
            .insert((
                TrackViewport,
                ScrollArea,
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    // `min: 0` lets the viewport shrink below its
                    // content so it clips and scrolls.
                    min_width: Val::Px(0.0),
                    min_height: Val::Px(0.0),
                    overflow: Overflow::scroll(),
                    ..default()
                },
            ))
            .with(|ui| {
                ui.elem(elem!(!TimelineTrack { width = 1.0 }))
                    .insert(TimelineContent)
                    .observe(on_track_press)
                    .observe(on_track_drag)
                    .observe(on_track_release)
                    .observe(on_track_click_release)
                    .observe(on_track_cancel)
                    .bind(
                        |track| track.width(),
                        resource_changed::<EditorState>(),
                        |world, node| match track_width(world, node) {
                            Val::Px(width) => width,
                            _ => 1.0,
                        },
                    )
                    .with(|ui| {
                        // The boxes get a container of their own, so
                        // the watcher's rebuild cannot take the
                        // playhead with it.
                        ui.elem(elem!(!Frame {}))
                            .insert(Node {
                                position_type: PositionType::Absolute,
                                top: Val::Px(TRACK_TOP_PADDING),
                                left: Val::Px(0.0),
                                ..default()
                            })
                            .watch(
                                value_changed(track_spans),
                                build_track_boxes,
                            );

                        ui.elem(elem!(!PlayheadLine)).bind(
                            |line| line.left(),
                            resource_changed::<MotionGfxManager>(),
                            |world, node| {
                                crate::px_for(current_time(
                                    world, node,
                                ))
                            },
                        );
                    });
            });
    });
}

/// The track viewport's scroll, read from `node`'s sibling.
fn viewport_scroll(world: &World, node: Entity) -> f32 {
    let Some(parent) = world.get::<ChildOf>(node) else {
        return 0.0;
    };
    let Some(siblings) = world.get::<Children>(parent.parent())
    else {
        return 0.0;
    };
    siblings
        .iter()
        .filter(|&sibling| {
            world.get::<TrackViewport>(sibling).is_some()
        })
        .find_map(|sibling| world.get::<ScrollPosition>(sibling))
        .map(|scroll| scroll.y)
        .unwrap_or(0.0)
}

/// Drag handler for the name-panel / track resize divider.
fn on_divider_drag(
    drag: On<Pointer<Drag>>,
    q_name_panel: Query<Entity, With<NamePanel>>,
    mut q_nodes: Query<&mut Node>,
) {
    let delta = drag.delta.x;
    if delta == 0.0 {
        return;
    }
    let Ok(name_panel) = q_name_panel.single() else {
        return;
    };
    let Ok(mut panel_node) = q_nodes.get_mut(name_panel) else {
        return;
    };
    if let Val::Px(w) = panel_node.width {
        let new_w = (w + delta).clamp(NAME_PANEL_MIN, NAME_PANEL_MAX);
        panel_node.width = Val::Px(new_w);
    }
}

/// `timeline.target_time()`, or zero if no timeline is focused yet.
fn current_time(world: &World, _: Entity) -> Duration {
    let state = world.resource::<EditorState>();
    let Some(id) = state.timeline else {
        return Duration::ZERO;
    };
    world
        .resource::<MotionGfxManager>()
        .get_timeline(&id)
        .map(|t| t.target_time())
        .unwrap_or(Duration::ZERO)
}

/// Track node width for the current duration, floored at 1px so a
/// zero-duration track still lays out.
fn track_width(world: &World, _: Entity) -> Val {
    let duration = world.resource::<EditorState>().duration;
    Val::Px(crate::px_for(duration).max(1.0))
}

/// Every track's duration, in order. The watcher's signal: a box only
/// needs rebuilding when a track is added, removed or re-timed.
fn track_spans(world: &World, _: Entity) -> Vec<Duration> {
    let state = world.resource::<EditorState>();
    let Some(id) = state.timeline else {
        return Vec::new();
    };
    world
        .resource::<MotionGfxManager>()
        .get_timeline(&id)
        .map(|timeline| {
            timeline
                .tracks()
                .iter()
                .map(|track| track.duration())
                .collect()
        })
        .unwrap_or_default()
}

/// One box per track, stacked top to bottom and scaled to duration.
fn build_track_boxes(ui: &mut BevyUi) {
    let spans = track_spans(ui.world, ui.parent());
    let fill = ui.world.resource::<EditorTheme>().palette.blue;

    for (index, duration) in spans.into_iter().enumerate() {
        let top = index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        let width = crate::px_for(duration).max(1.0);
        ui.elem(elem!(!Frame {
            width = Val::Px(width);
            height = Val::Px(TRACK_HEIGHT);
            radius = Val::Px(3.0);
            background = fill.with_alpha(0.35)
        }))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(top),
            left: Val::Px(0.0),
            width: Val::Px(width),
            height: Val::Px(TRACK_HEIGHT),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            ..default()
        });
    }
}

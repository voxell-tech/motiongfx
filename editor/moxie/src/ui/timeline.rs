//! The timeline panel: control bar (play/pause + time readout), name
//! column, divider and scrubbable track viewport.

use core::time::Duration;

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;
use bevy_motiongfx::prelude::MotionGfxManager;

use super::PANEL_PADDING;
use crate::block_layout::{self, Row};
use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_click_release,
    on_track_drag, on_track_press, on_track_release,
};
use crate::{EditorScene, EditorState};
use bevy_fynix::ElementMutExt;
use fynix_mock::{elem, val};
use moxie_ui::fynix::{
    Button, Frame, Icon, IconCursor, Label, LabelCursor, Panel,
    PanelCursor, PlayheadLine, PlayheadLineCursor, RawButtonCursor,
    TimelineTrack, TimelineTrackCursor,
};
use moxie_ui::reactive::{BevyUi, resource_changed, value_changed};
use moxie_ui::theme::EditorTheme;

const NAME_PANEL_WIDTH: f32 = 140.0;
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
    ui.elem(elem!(
        Panel,
        direction = FlexDirection::Column,
        padding = UiRect::bottom(px(PANEL_PADDING))
    ))
    .with(|ui| {
        control_bar(ui);
        track_area(ui);
    });
}

/// Play/pause + time readout.
fn control_bar(ui: &mut BevyUi) {
    ui.elem(elem!(
        Frame,
        width = percent(100),
        height = px(CONTROL_BAR_HEIGHT),
        align = AlignItems::Center,
        column_gap = px(12),
        padding = UiRect::horizontal(px(PANEL_PADDING))
    ))
    .with(|ui| {
        ui.elem(elem!(
            !Button,
            icon =
                val!(Icon, image = crate::icons::PLAY, size = px(14))
        ))
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

        ui.elem(elem!(Label, text = "0.00s")).bind(
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
    ui.elem(elem!(
        Panel,
        direction = FlexDirection::Row,
        padding = UiRect::horizontal(px(PANEL_PADDING))
    ))
    .with(|ui| {
        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            padding = UiRect::top(px(TRACK_TOP_PADDING)),
            scrolls = true
        ))
        .insert((
            NamePanel,
            Node {
                width: px(NAME_PANEL_WIDTH),
                height: percent(100),
                min_height: px(0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::top(px(TRACK_TOP_PADDING)),
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

        ui.elem(elem!(Frame, width = percent(100)))
            .insert((
                TrackViewport,
                ScrollArea,
                Node {
                    width: percent(100),
                    flex_grow: 1.0,
                    // `min: 0` lets the viewport shrink below its
                    // content so it clips and scrolls.
                    min_width: px(0),
                    min_height: px(0),
                    overflow: Overflow::scroll(),
                    ..default()
                },
            ))
            .with(|ui| {
                ui.elem(elem!(TimelineTrack, width = 1.0))
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
                        ui.elem(elem!(Frame,))
                            .insert(Node {
                                position_type: PositionType::Absolute,
                                top: px(TRACK_TOP_PADDING),
                                left: px(0),
                                ..default()
                            })
                            .watch(
                                value_changed(block_rows),
                                build_track_boxes,
                            );

                        ui.elem(elem!(PlayheadLine)).bind(
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
    px(crate::px_for(duration).max(1.0))
}

/// The editor scene's animation tree, flattened depth-first. The
/// watcher's signal: a box only needs rebuilding when a node is added,
/// removed, re-timed or re-nested.
fn block_rows(world: &World, _: Entity) -> Vec<Row> {
    world
        .get_resource::<EditorScene>()
        .map(|editor_scene| {
            block_layout::rows(&editor_scene.scene().0.animation)
        })
        .unwrap_or_default()
}

/// One box per row, stacked top to bottom and placed at its resolved
/// start time. Block-header rows (a [`Node::Block`](motiongfx_scene::block::Node::Block)'s
/// combinator) draw as a hollow outline spanning their children;
/// action leaves draw filled - the nesting itself reads from which
/// rows a header's outline encloses, not from indentation, since the
/// horizontal axis is already spoken for by time.
fn build_track_boxes(ui: &mut BevyUi) {
    let rows = block_rows(ui.world, ui.parent());
    let theme = ui.world.resource::<EditorTheme>();
    let action_fill = theme.palette.blue;
    let block_outline = theme.text_primary;

    for (index, row) in rows.into_iter().enumerate() {
        let top = index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        let left = crate::px_for(row.start);
        let width = crate::px_for(row.duration).max(1.0);

        let (background, border) = match row.combinator {
            Some(_) => (Color::NONE, block_outline.with_alpha(0.4)),
            None => (action_fill.with_alpha(0.35), Color::NONE),
        };

        ui.elem(elem!(
            Frame,
            width = px(width),
            height = px(TRACK_HEIGHT),
            radius = px(3),
            background = background
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: px(top),
            left: px(left),
            width: px(width),
            height: px(TRACK_HEIGHT),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        })
        .insert(BorderColor::all(border));
    }
}

//! The timeline panel: control bar (play/pause + time readout) and a
//! scrubbable track viewport, edge to edge - no name gutter, since a
//! block's own header box already carries its label.

use core::time::Duration;

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::ScrollArea;
use bevy_motiongfx::prelude::MotionGfxManager;

use super::PANEL_PADDING;
use crate::block_layout::{self, Placed};
use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_click_release,
    on_track_drag, on_track_press, on_track_release,
};
use crate::{EditorScene, EditorState};
use bevy_fynix::ElementMutExt;
use fynix_mock::{elem, val};
use moxie_ui::fynix::{
    Button, Frame, Icon, IconCursor, Label, LabelCursor, Panel,
    PlayheadLine, PlayheadLineCursor, RawButtonCursor, TimelineTrack,
    TimelineTrackCursor,
};
use moxie_ui::reactive::{BevyUi, resource_changed, value_changed};
use moxie_ui::theme::EditorTheme;

const CONTROL_BAR_HEIGHT: f32 = 40.0;
const TRACK_TOP_PADDING: f32 = 12.0;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(Component, Default, Clone)]
struct TrackViewport;

/// The scrubbable track, sized to the timeline's duration at
/// [`PIXELS_PER_SECOND`](crate::PIXELS_PER_SECOND). Holds the track
/// boxes and the playhead; scrubbing comes from pointer observers on
/// it, so a drag can only start from a press that lands inside.
#[derive(Component, Default, Clone)]
pub(crate) struct TimelineContent;

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

/// The scrollable track viewport, filling the whole panel width.
fn track_area(ui: &mut BevyUi) {
    ui.elem(elem!(Frame, width = percent(100)))
        .insert((
            TrackViewport,
            ScrollArea,
            Node {
                width: percent(100),
                flex_grow: 1.0,
                // `min: 0` lets the viewport shrink below its content
                // so it clips and scrolls.
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
                    // The boxes get a container of their own, so the
                    // watcher's rebuild cannot take the playhead with
                    // it.
                    ui.elem(elem!(Frame,))
                        .insert(Node {
                            position_type: PositionType::Absolute,
                            top: px(TRACK_TOP_PADDING),
                            left: px(0),
                            ..default()
                        })
                        .watch(
                            value_changed(block_placements),
                            build_block_boxes,
                        );

                    ui.elem(elem!(PlayheadLine)).bind(
                        |line| line.left(),
                        resource_changed::<MotionGfxManager>(),
                        |world, node| {
                            crate::px_for(current_time(world, node))
                        },
                    );
                });
        });
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

/// The editor scene's animation tree, laid out as nested boxes. The
/// watcher's signal: a box only needs rebuilding when a node is added,
/// removed, re-timed or re-nested.
fn block_placements(world: &World, _: Entity) -> Vec<Placed> {
    world
        .get_resource::<EditorScene>()
        .map(|editor_scene| {
            block_layout::layout(&editor_scene.scene().0.animation)
        })
        .unwrap_or_default()
}

/// One box per placement: a block's header box drawn as a hollow
/// outline enclosing its children, an action leaf drawn filled. A
/// `dotted` piece (an `Any`'s losing branch, past the group's official
/// end) draws ghosted instead - faint enough to read as "still
/// playing, but no longer part of this group's timing" - and skips its
/// own label, since the solid piece right before it already showed one.
fn build_block_boxes(ui: &mut BevyUi) {
    let placements = block_placements(ui.world, ui.parent());
    let theme = ui.world.resource::<EditorTheme>();
    let action_fill = theme.palette.blue;
    let block_outline = theme.text_primary;

    for placed in placements {
        let is_block = placed.label.is_some();
        let ghost = if placed.dotted { 0.45 } else { 1.0 };
        let (background, border) = if is_block {
            (
                block_outline.with_alpha(0.04 * ghost),
                block_outline.with_alpha(0.4 * ghost),
            )
        } else {
            (action_fill.with_alpha(0.35 * ghost), Color::NONE)
        };

        ui.elem(elem!(
            Frame,
            width = px(placed.w),
            height = px(placed.h),
            radius = px(3),
            background = background
        ))
        .insert(Node {
            position_type: PositionType::Absolute,
            top: px(placed.y),
            left: px(placed.x),
            width: px(placed.w),
            height: px(placed.h),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        })
        .insert(BorderColor::all(border))
        .with(|ui| {
            if placed.dotted {
                return;
            }
            let Some(label) = placed.label.clone() else {
                return;
            };
            ui.elem(elem!(
                Label,
                text = label,
                size = 10.0,
                color = Some(block_outline.with_alpha(0.8))
            ))
            .insert(Node {
                position_type: PositionType::Absolute,
                top: px(2),
                left: px(4),
                ..default()
            });
        });
    }
}

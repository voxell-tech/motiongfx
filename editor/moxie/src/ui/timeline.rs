//! The timeline panel: control bar (play/pause + time readout) and a
//! scrubbable track viewport, edge to edge - no name gutter, since a
//! block's own header box already carries its label.

use core::time::Duration;

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_motiongfx::prelude::MotionGfxManager;

use super::PANEL_PADDING;
use crate::block_layout::{self, Placed};
use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_click_release,
    on_track_drag, on_track_press, on_track_release,
};
use crate::{EditorScene, EditorState, SelectedAction};
use bevy_fynix::ElementMutExt;
use fynix_mock::{elem, val};
use moxie_ui::fynix::{
    Button, ButtonElemCursor, Frame, Icon, IconCursor, Label,
    LabelCursor, Panel, PlayheadLine, PlayheadLineCursor, ScrollArea,
    TimelineAction, TimelineBlock,
};
use moxie_ui::reactive::{BevyUi, resource_changed, value_changed};
use moxie_ui::theme::EditorTheme;

const CONTROL_BAR_HEIGHT: f32 = 40.0;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(Component, Default, Clone)]
struct TrackViewport;

/// The timeline panel, as kernel nodes.
///
/// Each reactive field binds at the node that owns it, which is why
/// this is a builder rather than a `bsn!` tree: the play/pause icon,
/// time label and friends have to be `NodeMut`s to carry their own
/// binds.
pub(super) fn panel(ui: &mut BevyUi) {
    ui.elem(elem!(Panel, direction = FlexDirection::Column))
        .with(control_bar)
        .with(track_area);
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

/// The scrollable track viewport, filling the whole panel width, with
/// the playhead floating over it as a sibling - not a descendant, so
/// it's neither scrolled nor clipped by the [`ScrollArea`].
fn track_area(ui: &mut BevyUi) {
    ui.elem(elem!(Frame, width = percent(100), flex_grow = 1.0))
        .observe(on_track_press)
        .observe(on_track_drag)
        .observe(on_track_release)
        .observe(on_track_click_release)
        .observe(on_track_cancel)
        .with(|ui| {
            ui.elem(elem!(PlayheadLine)).bind(
                |line| line.left(),
                resource_changed::<MotionGfxManager>(),
                |world, node| {
                    crate::px_for(current_time(world, node))
                },
            );
        })
        .with(|ui| {
            ui.elem(elem!(ScrollArea, width = percent(100)))
                .insert(TrackViewport)
                .watch(value_changed(block_view), build_block_boxes);
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

/// The editor scene's animation tree, laid out as nested boxes.
fn block_placements(world: &World, _: Entity) -> Vec<Placed> {
    world
        .get_resource::<EditorScene>()
        .map(|editor_scene| {
            block_layout::layout(&editor_scene.scene().0.animation)
        })
        .unwrap_or_default()
}

/// The boxes plus which one (if any) is selected - the watcher's
/// signal, so a box only needs rebuilding when a node is added,
/// removed, re-timed, re-nested, or selection moves onto or off it.
fn block_view(
    world: &World,
    node: Entity,
) -> (Vec<Placed>, Option<Vec<usize>>) {
    let selected = world
        .get_resource::<SelectedAction>()
        .and_then(|s| s.0.clone());
    (block_placements(world, node), selected)
}

/// One box per placement: a block's header - a [`TimelineBlock`],
/// which owns its own label - or an action leaf's own [`TimelineAction`],
/// which lights up under the cursor and outlines in the theme's
/// accent when [`SelectedAction`] names its path - clicking it writes
/// that path in.
///
/// An `Any` block's box (and any ancestor whose visual extent it
/// bleeds into) is already sized to its losing branch's full
/// duration - see [`block_layout::layout`] - so nothing here needs to
/// clip or fade anything to keep a slower action fully visible.
fn build_block_boxes(ui: &mut BevyUi) {
    let (placements, selected) = block_view(ui.world, ui.parent());
    let theme = ui.world.resource::<EditorTheme>();
    let action_fill = theme.palette.blue;
    let block_outline = theme.text_primary;
    let accent = theme.accent;

    for placed in placements {
        let is_selected = selected.as_ref() == Some(&placed.path);

        match placed.label {
            Some(label) => {
                ui.elem(elem!(
                    TimelineBlock,
                    label = val!(
                        Label,
                        text = label,
                        size = 10.0,
                        color = Some(block_outline.with_alpha(0.8))
                    ),
                    top = placed.y,
                    left = placed.x,
                    width = placed.w,
                    height = placed.h,
                    background = block_outline.with_alpha(0.04),
                    border = block_outline.with_alpha(0.4)
                ));
            }
            // An action leaf's own element: position, colors and
            // selection are all typed fields, and it owns its
            // pointer cursor and hover/press tint itself.
            None => {
                let path = placed.path.clone();
                ui.elem(elem!(
                    TimelineAction,
                    top = placed.y,
                    left = placed.x,
                    width = placed.w,
                    height = placed.h,
                    fill = action_fill.with_alpha(0.35),
                    border = if is_selected {
                        accent
                    } else {
                        Color::NONE
                    },
                    selected = is_selected
                ))
                .observe(
                    move |_: On<Activate>,
                          mut selected: ResMut<SelectedAction>| {
                        selected.0 = Some(path.clone());
                    },
                );
            }
        }
    }
}

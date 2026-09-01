//! The timeline panel: control bar (play/pause + time readout) and a
//! scrubbable track viewport, edge to edge. No name gutter: a
//! block's own header box already carries its label.

use core::time::Duration;
use std::collections::BTreeSet;

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, ScrollArea as ScrollAreaBehavior};
use bevy_motiongfx::prelude::MotionGfxManager;

use super::PANEL_PADDING;
use crate::block_layout::{self, Placed};
use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_click_release,
    on_track_drag, on_track_press, on_track_release,
};
use crate::zoom::{FitTimeline, on_track_scroll};
use crate::{
    EditorScene, EditorState, SelectedAction, TimelineView, time_axis,
};
use bevy_fynix::WorldEntityMut;
use fynix::WorldNodeRef;
use fynix::composer::Composer;
use fynix::ui::ElementHandle;
use fynix::{elem, val};
use moxie_ui::elements::{
    Button, ButtonElemCursor, Frame, Icon, IconCursor, Label,
    LabelCursor, Panel, PlayheadLine, PlayheadLineCursor, ScrollArea,
    TimeLabel, TimeTick, TimelineAction, TimelineActionCursor,
    TimelineBlock, TintButton,
};
use moxie_ui::fold::{CHEVRON_OPEN, CHEVRON_SHUT};
use moxie_ui::motion::MotionExt;
use moxie_ui::reactive::{
    BevyHost, BevyUi, resource_changed, value_changed,
};

/// Folded blocks, by path.
#[derive(Resource, Default, Clone, PartialEq)]
pub(super) struct BlockFoldState(BTreeSet<Vec<usize>>);

fn toggle_folded(world: &mut World, path: &[usize]) {
    let mut state = world.resource_mut::<BlockFoldState>();
    if !state.0.remove(path) {
        state.0.insert(path.to_vec());
    }
}

const CONTROL_BAR_HEIGHT: f32 = 40.0;
const TIME_AXIS_HEIGHT: f32 = 24.0;
const MAJOR_TICK: f32 = 8.0;
const MINOR_TICK: f32 = 4.0;
const LABEL_SIZE: f32 = 10.0;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(Component, Default, Clone)]
pub(crate) struct TrackViewport;

/// The timeline panel, as kernel nodes.
///
/// Each reactive field binds at the node that owns it, so the
/// play/pause icon, time label and friends have to be `NodeMut`s to
/// carry their own binds. That is why this is a composer, not a
/// `bsn!` tree.
pub(super) struct TimelinePanel;

impl Composer<BevyHost> for TimelinePanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        ui.elem(elem!(Panel))
            .with(|ui| {
                ui.elem(elem!(
                    Frame,
                    width = percent(100),
                    height = percent(100),
                    direction = FlexDirection::Column
                ))
                .with(|ui| {
                    ui.compose(ControlBar);
                    ui.compose(TrackArea);
                });
            })
            .handle()
    }
}

/// Play/pause + time readout.
struct ControlBar;

impl Composer<BevyHost> for ControlBar {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
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
                icon = val!(
                    Icon,
                    image = crate::icons::PLAY,
                    size = px(14)
                )
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
                |WorldNodeRef { world, .. }| {
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
                |WorldNodeRef {
                     world,
                     node: entity,
                 }| {
                    format!(
                        "{:.2}s",
                        current_time(world, entity).as_secs_f32()
                    )
                },
            );

            // Fit button.
            ui.elem(elem!(Frame, flex_grow = 1.0f32));

            ui.elem(elem!(
                !Button,
                label = val!(Label, text = "Fit"),
                width = px(44),
                height = px(24)
            ))
            .observe(
                |mut click: On<Pointer<Click>>,
                 mut commands: Commands| {
                    click.propagate(false);
                    commands.trigger(FitTimeline);
                },
            );
        })
        .handle()
    }
}

/// The time axis ruler above the tracks.
struct TimeAxis;

impl Composer<BevyHost> for TimeAxis {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        ui.elem(elem!(
            Frame,
            width = percent(100),
            height = px(TIME_AXIS_HEIGHT),
        ))
        .watch(value_changed(axis_view), build_ticks)
        .handle()
    }
}

/// The time axis's width and the view it draws, so a change to
/// either retriggers the watch. Width is rounded so sub-pixel
/// jitter cannot.
fn axis_view(world: &World, node: Entity) -> (u32, TimelineView) {
    let width = world
        .get::<ComputedNode>(node)
        .map(|computed| {
            (computed.size().x * computed.inverse_scale_factor())
                as u32
        })
        .unwrap_or(0);

    (width, *world.resource::<TimelineView>())
}

fn build_ticks(ui: &mut BevyUi) {
    let (width, view) = axis_view(ui.world, ui.parent());
    let color = ui.theme.text_muted;
    let marks = time_axis::ticks(&view, width as f32);

    for tick in marks {
        let major = tick.label.is_some();
        ui.elem(elem!(
            TimeTick,
            x = tick.x,
            height = if major { MAJOR_TICK } else { MINOR_TICK },
            color = color.with_alpha(if major { 0.6 } else { 0.3 })
        ));

        if let Some(text) = tick.label {
            ui.elem(elem!(
                TimeLabel,
                x = tick.x,
                label = val!(
                    Label,
                    text = text,
                    size = LABEL_SIZE,
                    wrap = false,
                    color = Some(color.with_alpha(0.7))
                )
            ));
        }
    }
}

/// The scrollable track viewport, filling the whole panel width. The
/// playhead floats over it as a sibling, not a descendant, so the
/// [`ScrollArea`] neither scrolls nor clips it.
struct TrackArea;

impl Composer<BevyHost> for TrackArea {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column,
            flex_grow = 1.0f32
        ))
        .observe(on_track_press)
        .observe(on_track_drag)
        .observe(on_track_release)
        .observe(on_track_click_release)
        .observe(on_track_cancel)
        .observe(on_track_scroll)
        .with(|ui| {
            ui.elem(elem!(PlayheadLine)).bind(
                |line| line.left(),
                resource_changed::<MotionGfxManager>(),
                |WorldNodeRef { world, node }| {
                    world
                        .resource::<TimelineView>()
                        .x_from_time(current_time(world, node))
                },
            );
        })
        .with(|ui| {
            ui.compose(TimeAxis);
        })
        .with(|ui| {
            ui.elem(elem!(
                ScrollArea,
                width = percent(100),
                flex_grow = 1.0f32
            ))
            .insert(TrackViewport)
            .remove::<ScrollAreaBehavior>()
            .watch(value_changed(block_view), build_block_boxes);
        })
        .handle()
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

/// The editor scene's animation tree, laid out as nested boxes.
fn block_placements(world: &World, _: Entity) -> Vec<Placed> {
    let view = *world.resource::<TimelineView>();
    let empty = BTreeSet::new();
    let folded = world
        .get_resource::<BlockFoldState>()
        .map_or(&empty, |state| &state.0);

    world
        .get_resource::<EditorScene>()
        .map(|editor_scene| {
            block_layout::layout(
                &editor_scene.scene().0.animation,
                view,
                folded,
            )
        })
        .unwrap_or_default()
}

/// The boxes plus which one, if any, is selected. The watcher's
/// signal: a box rebuilds only when a node is added, removed,
/// re-timed, re-nested, or selection moves onto or off it.
fn block_view(
    world: &World,
    node: Entity,
) -> (Vec<Placed>, Option<Vec<usize>>) {
    let selected = world
        .get_resource::<SelectedAction>()
        .and_then(|s| s.0.clone());
    (block_placements(world, node), selected)
}

/// One box per placement: a block's header ([`TimelineBlock`], which
/// owns its own label), or an action leaf's own [`TimelineAction`].
/// Either outlines in the theme's accent when [`SelectedAction`] names
/// its path, and clicking either writes that path in; only the action
/// also lights up under the cursor.
fn build_block_boxes(ui: &mut BevyUi) {
    let (placements, selected) = block_view(ui.world, ui.parent());
    let theme = ui.theme;

    for placed in placements {
        let is_selected = selected.as_ref() == Some(&placed.path);

        match placed.label {
            Some(label) => {
                let path = placed.path.clone();
                ui.elem(elem!(
                    TimelineBlock,
                    label = val!(
                        Label,
                        text = label,
                        size = 10.0f32,
                        color = Some(theme.text_primary.with_alpha(0.8))
                    ),
                    top = placed.y,
                    left = placed.x,
                    width = placed.w,
                    height = placed.h,
                    background = theme.text_primary.with_alpha(0.04),
                    border = if is_selected {
                        theme.accent
                    } else {
                        theme.text_primary.with_alpha(0.4)
                    }
                ))
                .observe(
                    move |_: On<Activate>,
                          mut selected: ResMut<SelectedAction>| {
                        selected.0 = Some(path.clone());
                    },
                );

                // Its own element, absolutely positioned over the
                // block's top-left corner rather than nested in
                // `TimelineBlock`: a nested row would need
                // `AlignItems::Center` to line up with the label,
                // which centers in the whole block's height instead
                // of just the header strip.
                let path = placed.path.clone();
                let folded = placed.folded;
                ui.elem(elem!(
                    Frame,
                    position = PositionType::Absolute,
                    inset = UiRect::new(
                        px(placed.x),
                        auto(),
                        px(placed.y),
                        auto()
                    ),
                    width = px(12),
                    height = px(12)
                ))
                .with(move |ui| {
                    ui.elem(elem!(
                        !TintButton::default(),
                        radius = px(3),
                        icon = val!(
                            Icon,
                            image = moxie_ui::icons::CHEVRON,
                            size = px(7),
                            color = theme.text_primary.with_alpha(0.6),
                            rotation = if folded {
                                CHEVRON_SHUT
                            } else {
                                CHEVRON_OPEN
                            }
                        )
                    ))
                    .observe(
                        move |_: On<Activate>, mut commands: Commands| {
                            let path = path.clone();
                            commands.queue(move |world: &mut World| {
                                toggle_folded(world, &path);
                            });
                        },
                    );
                });
            }
            // An action leaf's own element: position, colors and
            // selection are all typed fields, and it owns its
            // pointer cursor and hover/press tint itself.
            None => {
                let path = placed.path.clone();
                // A draft has no subject/field yet, so its clip reads
                // as an empty slot in the critical color, rather than
                // a real action's fill.
                let fill = if placed.draft {
                    theme.critical.with_alpha(0.12)
                } else {
                    theme.palette.blue.with_alpha(0.35)
                };
                ui.elem(elem!(
                    TimelineAction,
                    label = val!(
                        Label,
                        text = placed
                            .name
                            .unwrap_or_else(|| if placed.draft {
                                "Draft".to_string()
                            } else {
                                String::new()
                            }),
                        size = 10.0f32,
                        color = Some(if placed.draft {
                            theme.critical.with_alpha(0.9)
                        } else {
                            theme.palette.blue.with_alpha(0.9)
                        })
                    ),
                    top = placed.y,
                    left = placed.x,
                    width = placed.w,
                    height = placed.h,
                    fill = fill,
                    border = if is_selected {
                        theme.accent
                    } else if placed.draft {
                        theme.critical.with_alpha(0.5)
                    } else {
                        Color::NONE
                    },
                    selected = is_selected
                ))
                .lit(|action| action.fill(), theme.clip_hover, theme.clip_press)
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

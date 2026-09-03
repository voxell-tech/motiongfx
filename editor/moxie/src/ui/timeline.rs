//! The timeline panel: control bar (play/pause + time readout) and a
//! scrubbable track viewport, edge to edge. No name gutter: a
//! block's own header box already carries its label.

mod drag;
mod pattern;

pub(crate) use drag::{Dragging, cancel_on_escape};
pub(crate) use pattern::DelayPattern;

use core::time::Duration;
use std::collections::BTreeSet;

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
use fynix::elem;
use fynix::ui::ElementHandle;
use moxie_ui::elements::{
    Button, ButtonCursor, Frame, FrameCursor, GhostButton, Icon,
    IconCursor, Label, LabelCursor, Panel, PlayheadLine,
    PlayheadLineCursor, ScrollArea, TimeLabel, TimeTick,
    TimelineAction, TimelineActionCursor, TimelineBlock, TimelineGap,
    TintButton,
};
use moxie_ui::fold::{CHEVRON_OPEN, CHEVRON_SHUT};
use moxie_ui::motion::MotionExt;
use moxie_ui::reactive::{
    BevyUi, FynixHost, resource_changed, value_changed,
};

/// Folded blocks, by path.
#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct BlockFoldState(BTreeSet<Vec<usize>>);

impl BlockFoldState {
    /// The folded paths this holds.
    pub(crate) fn paths(&self) -> &BTreeSet<Vec<usize>> {
        &self.0
    }
}

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

impl Composer<FynixHost> for TimelinePanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Panel> {
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

impl Composer<FynixHost> for ControlBar {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
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
                Button,
                icon = elem!(
                    Icon,
                    image = crate::icons::PLAY,
                    size = px(14)
                )
            ))
            .observe(|_: On<Activate>, mut commands: Commands| {
                commands.trigger(TogglePlayback);
            })
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
                Button,
                label = elem!(Label, text = "Fit"),
                width = px(44),
                height = px(24)
            ))
            .observe(
                |_: On<Activate>, mut commands: Commands| {
                    commands.trigger(FitTimeline);
                },
            );
        })
        .handle()
    }
}

/// The time axis ruler above the tracks.
struct TimeAxis;

impl Composer<FynixHost> for TimeAxis {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
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
    let color = ui.theme.color.text_dim;
    let text_size = ui.theme.text.small;
    let marks = time_axis::ticks(&view, width as f32);

    for tick in marks {
        let major = tick.label.is_some();
        ui.elem(elem!(
            TimeTick,
            x = px(tick.x),
            height = px(if major { MAJOR_TICK } else { MINOR_TICK }),
            color = color.with_alpha(if major { 0.6 } else { 0.3 })
        ));

        if let Some(text) = tick.label {
            ui.elem(elem!(
                TimeLabel,
                x = px(tick.x),
                label = elem!(
                    Label,
                    text = text,
                    size = text_size,
                    wrap = false,
                    color = color.with_alpha(0.7)
                )
            ));
        }
    }
}

/// The scrollable track viewport, filling the whole panel width. The
/// playhead floats over it as a sibling, not a descendant, so the
/// [`ScrollArea`] neither scrolls nor clips it.
struct TrackArea;

impl Composer<FynixHost> for TrackArea {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
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
                    px(world
                        .resource::<TimelineView>()
                        .x_from_time(current_time(world, node)))
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

/// A block's header: its name (or combinator, if unnamed) beside its
/// fold chevron, clickable to select - the chevron alone toggles the
/// fold.
struct BlockHeader {
    path: Vec<usize>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    folded: bool,
    label: String,
    is_selected: bool,
}

impl Composer<FynixHost> for BlockHeader {
    type Element = TimelineBlock;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, TimelineBlock> {
        let Self {
            path,
            x,
            y,
            w,
            h,
            folded,
            label,
            is_selected,
        } = self;
        let theme = ui.theme;
        let default_color = theme.color.text;
        let selected_color = theme.palette.purple;

        let block_color = if is_selected {
            selected_color
        } else {
            default_color
        };

        let chevron_color = theme.color.text_faint;
        let label_color = theme.color.text.with_alpha(0.8);

        let mut header = ui.elem(elem!(
            TimelineBlock,
            top = px(y),
            left = px(x),
            width = px(w),
            height = px(h),
            background = block_color.with_alpha(0.03),
            border = block_color.with_alpha(0.5)
        ));
        header.insert(drag::BoxPath(path.clone())).with(move |ui| {
            ui.elem(elem!(
                !GhostButton,
                width = percent(100),
                height = px(18),
                justify = JustifyContent::FlexStart,
                padding = UiRect::axes(px(4), px(2)),
                radius = Val::ZERO,
                column_gap = px(4)
            ))
            .observe({
                let path = path.clone();
                move |_: On<Activate>,
                      mut selected: ResMut<SelectedAction>| {
                    selected.0 = Some(path.clone());
                }
            })
            .with(move |ui| {
                chevron(ui, path, folded, chevron_color);
                ui.elem(elem!(
                    Label,
                    text = label,
                    wrap = false,
                    color = label_color
                ));
            });
        });

        header.handle()
    }
}

/// One box per placement: a block's header ([`BlockHeader`]), or an
/// action leaf's own [`TimelineAction`]. Either outlines in the
/// theme's accent when [`SelectedAction`] names its path, and
/// clicking either writes that path in; only the action also lights
/// up under the cursor.
fn build_block_boxes(ui: &mut BevyUi) {
    let (placements, selected) = block_view(ui.world, ui.parent());
    let theme = ui.theme;
    let pattern = ui.world.resource::<DelayPattern>().0.clone();

    for placed in placements {
        let is_selected = selected.as_ref() == Some(&placed.path);

        // Spawned at zero width even with no delay yet, so a live
        // drag that opens one up has an entity already in place to
        // grow.
        if !placed.path.is_empty() {
            let gap_x = placed.gap_x.unwrap_or(placed.x);
            let image = pattern.clone();
            ui.elem(elem!(
                TimelineGap,
                top = px(placed.y),
                left = px(gap_x),
                width = px(placed.x - gap_x),
                height = px(placed.h),
                image = image,
                color = theme.color.text_dim.with_alpha(0.35)
            ))
            .insert(drag::GapPath(placed.path.clone()));
        }

        match placed.label {
            Some(label) => {
                let path = placed.path.clone();
                ui.compose(BlockHeader {
                    path: path.clone(),
                    x: placed.x,
                    y: placed.y,
                    w: placed.w,
                    h: placed.h,
                    folded: placed.folded,
                    label,
                    is_selected,
                });
                // The root's box has no `delay` of its own to drag -
                // it always starts at zero.
                if !path.is_empty() {
                    edge_handle(
                        ui,
                        path,
                        drag::Kind::Move,
                        placed.x,
                        placed.y,
                        placed.h,
                    );
                }
            }
            // An action leaf's own element: position, colors and
            // selection are all typed fields, and it owns its
            // pointer cursor and hover/press tint itself.
            None => {
                let path = placed.path.clone();
                let label =
                    placed.name.clone().unwrap_or_else(|| {
                        if placed.draft {
                            "Draft".to_string()
                        } else {
                            String::new()
                        }
                    });
                // A draft has no subject/field yet, so its clip reads
                // as an empty slot in the critical color, rather than
                // a real action's fill.
                let fill = if placed.draft {
                    theme.color.critical.with_alpha(0.5)
                } else {
                    theme.color.clip
                };
                let border = if is_selected {
                    theme.color.accent
                } else if placed.draft {
                    theme.color.critical.with_alpha(0.5)
                } else {
                    Color::NONE
                };
                let mut clip = ui.elem(elem!(
                    TimelineAction,
                    label = elem!(
                        Label,
                        text = label.clone(),
                        size = theme.text.small,
                        color = if placed.draft {
                            theme.color.critical.with_alpha(0.9)
                        } else {
                            theme.palette.blue.with_alpha(0.9)
                        }
                    ),
                    top = px(placed.y),
                    left = px(placed.x),
                    width = px(placed.w),
                    height = px(placed.h),
                    fill = fill,
                    border = border,
                    selected = is_selected
                ));
                clip.insert(drag::BoxPath(placed.path.clone()))
                    .lit(
                        |action| action.fill(),
                        theme.color.clip_hover,
                        theme.color.clip_press,
                    )
                    .observe({
                        let path = path.clone();
                        move |_: On<Activate>,
                              mut selected: ResMut<SelectedAction>| {
                            selected.0 = Some(path.clone());
                        }
                    });
                edge_handle(
                    ui,
                    path.clone(),
                    drag::Kind::Move,
                    placed.x,
                    placed.y,
                    placed.h,
                );
                edge_handle(
                    ui,
                    path,
                    drag::Kind::Resize,
                    placed.x + placed.w - drag::EDGE_HANDLE_PX,
                    placed.y,
                    placed.h,
                );
            }
        }
    }
}

/// A block's fold toggle: positioned relative to its own corner
/// rather than the placement's absolute coordinates, so it moves for
/// free with whatever a live drag does to the block's own `Node`.
fn chevron(
    ui: &mut BevyUi,
    path: Vec<usize>,
    folded: bool,
    color: Color,
) {
    ui.elem(elem!(
        !TintButton::default(),
        icon = elem!(
            Icon,
            image = moxie_ui::icons::CHEVRON,
            size = px(7),
            color = color,
            rotation =
                if folded { CHEVRON_SHUT } else { CHEVRON_OPEN }
        )
    ))
    .observe(move |_: On<Activate>, mut commands: Commands| {
        let path = path.clone();
        commands.queue(move |world: &mut World| {
            toggle_folded(world, &path);
        });
    });
}

/// A thin, absolutely positioned strip at one edge of a box, wired to
/// `kind` via [`drag::edge`].
fn edge_handle(
    ui: &mut BevyUi,
    path: Vec<usize>,
    kind: drag::Kind,
    x: f32,
    y: f32,
    h: f32,
) {
    let accent = ui.theme.color.accent;
    let mut handle = ui.elem(elem!(
        Frame,
        position = PositionType::Absolute,
        inset = UiRect::new(px(x), auto(), px(y), auto()),
        width = px(drag::EDGE_HANDLE_PX),
        height = px(h)
    ));
    handle.lit(
        |frame| frame.background(),
        accent.with_alpha(0.35),
        accent.with_alpha(0.6),
    );
    drag::edge(&mut handle, path, kind);
}

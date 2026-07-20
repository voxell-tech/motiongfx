//! The editor's UI component markers and the static `bsn!` scene tree
//! (panel, control bar, name column, scroll viewport), plus the
//! startup system that spawns it against a dedicated UI camera.

use std::sync::Arc;

use bevy::camera::Hdr;
use bevy::camera::visibility::RenderLayers;
use bevy::picking::events::{Click, Drag, Pointer};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::settings::SaveSettingsSync;
use bevy::ui::widget::ImageNode;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};
use bevy::ui_widgets::{ControlOrientation, ScrollArea};
use bevy_motiongfx::prelude::MotionGfxManager;

use crate::playback::{
    TogglePlayback, on_track_cancel, on_track_drag, on_track_press,
    on_track_release,
};
use crate::{
    CONTROL_BAR_HEIGHT, EditorSettings, EditorState, NAME_PANEL_MAX,
    NAME_PANEL_MIN, NAME_PANEL_WIDTH, PANEL_PADDING, PreviewImage,
    TRACK_GAP, TRACK_HEIGHT, TRACK_TOP_PADDING,
};
use motiongfx_editor_ui::dock::{
    DockAreaStyle, DockLeaf, DockNode, DockTree,
    DockWindowDescriptor, Edge, WindowRegistry, dock,
};
use motiongfx_editor_ui::glass::{
    Glass, bind_backdrop, glass_button,
};
use motiongfx_editor_ui::inspector::inspector_fields;
use motiongfx_editor_ui::reactive::{
    BevyNodeMutExt, BevyUi, BevyUiExt, KernelRoot, resource_changed,
    value_changed,
};
use motiongfx_editor_ui::theme::EditorTheme;
use motiongfx_editor_ui::{
    Divider, label, playhead_line, timeline_track,
};

/// Marker for the timeline panel node (fills its dock area).
#[derive(Component, Default, Clone)]
pub(crate) struct EditorPanel;

/// Marks the UI camera (which owns the window). Every other (scene)
/// camera is retargeted to the offscreen preview image; see
/// [`retarget_scene_cameras`].
///
/// [`retarget_scene_cameras`]: crate::view::retarget_scene_cameras
#[derive(Component, Default, Clone)]
pub(crate) struct TrackViewportCamera;

/// The area above the panel that holds the letterboxed preview image.
#[derive(Component, Default, Clone)]
pub(crate) struct PreviewArea;

/// The [`ImageNode`] displaying the offscreen composition; letterboxed
/// into [`PreviewArea`] by a bind on its parent's computed size (see
/// [`preview_fit`](crate::view::preview_fit)).
#[derive(Component, Default, Clone)]
pub(crate) struct PreviewDisplay;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(Component, Default, Clone)]
pub(crate) struct TrackViewport;

/// The scrubbable track, sized to the timeline's duration at
/// [`PIXELS_PER_SECOND`](crate::PIXELS_PER_SECOND). Holds the track
/// boxes and the playhead; scrubbing comes from pointer observers on
/// it, so a drag can only start from a press that lands inside.
#[derive(Component, Default, Clone)]
pub(crate) struct TimelineContent;

#[derive(Component, Default, Clone)]
pub(crate) struct NamePanel;

#[derive(Component, Default, Clone)]
pub(crate) struct Playhead;

#[derive(Component, Default, Clone)]
pub(crate) struct PlayPauseLabel;

#[derive(Component, Default, Clone)]
pub(crate) struct TimeLabel;

#[derive(Component, Default, Clone)]
pub(crate) struct SettingsSaveLabel;

/// The timeline panel, as kernel nodes.
///
/// Each reactive field binds at the node that owns it, which is why
/// this is a builder rather than a `bsn!` tree: `Playhead`, `TimeLabel`
/// and friends have to be `NodeMut`s to carry their own binds.
pub(crate) fn timeline_panel(ui: &mut BevyUi) {
    ui.bsn(bsn! {
        EditorPanel
        Node {
            width: Val::Percent(100.0),
            // Fill the dock area; the dock split handle resizes it.
            flex_grow: 1.0,
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::bottom(Val::Px(PANEL_PADDING)),
        }
        template_value(Glass::Panel)
    })
    .with(|ui| {
        control_bar(ui);
        track_area(ui);
    });
}

/// Play/pause + time readout.
fn control_bar(ui: &mut BevyUi) {
    ui.bsn(bsn! {
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(CONTROL_BAR_HEIGHT),
            flex_shrink: 0.0,
            align_items: AlignItems::Center,
            column_gap: Val::Px(12.0),
            padding: UiRect::horizontal(Val::Px(PANEL_PADDING)),
        }
    })
    .with(|ui| {
        ui.bsn(bsn! {
            glass_button()
            on(|mut click: On<Pointer<Click>>,
                mut commands: Commands| {
                click.propagate(false);
                commands.trigger(TogglePlayback);
            })
            Node {
                width: Val::Px(84.0),
                height: Val::Px(26.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(Val::Px(6.0)),
            }
        })
        .with(|ui| {
            ui.bsn(label::<PlayPauseLabel>("Play")).bind::<Text>(
                resource_changed::<EditorState>(),
                |world, _| {
                    Text::new(
                        if world.resource::<EditorState>().is_playing
                        {
                            "Pause"
                        } else {
                            "Play"
                        },
                    )
                },
            );
        });

        ui.bsn(label::<TimeLabel>("0.00s")).bind::<Text>(
            resource_changed::<MotionGfxManager>(),
            |world, entity| {
                Text::new(format!(
                    "{:.2}s",
                    current_time(world, entity)
                ))
            },
        );
    });
}

/// Name column | divider | scroll viewport.
fn track_area(ui: &mut BevyUi) {
    ui.bsn(bsn! {
        Node {
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            // Allow this flex item to shrink below its content height
            // so the viewport below can clip and scroll (flex items
            // default to `min-height: auto`).
            min_height: Val::Px(0.0),
            flex_direction: FlexDirection::Row,
            padding: UiRect::horizontal(Val::Px(PANEL_PADDING)),
        }
    })
    .with(|ui| {
        ui.bsn(bsn! {
            NamePanel
            ScrollPosition
            Node {
                width: Val::Px(NAME_PANEL_WIDTH),
                height: Val::Percent(100.0),
                min_height: Val::Px(0.0),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                padding: UiRect::top(Val::Px(TRACK_TOP_PADDING)),
            }
            template_value(Glass::Panel)
        })
        // Locked to the track viewport, found as a sibling: the
        // builder cannot know its entity id yet.
        .bind_field::<ScrollPosition, _>(
            value_changed(viewport_scroll),
            viewport_scroll,
            |scroll, y| scroll.y = y,
        );

        ui.bsn(bsn! {
            @Divider {
                @thickness: Val::Px(4.0),
                @orientation: ControlOrientation::Vertical
            }
            on(on_divider_drag)
        });

        ui.bsn(bsn! {
            TrackViewport
            ScrollArea
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                // `min: 0` lets the viewport shrink below its
                // (tall/wide) content so it clips and scrolls.
                min_width: Val::Px(0.0),
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll(),
            }
            template_value(Glass::Panel)
        })
        .with(|ui| {
            ui.bsn(bsn! {
                TimelineContent
                timeline_track(1.0)
                on(on_track_press)
                on(on_track_drag)
                on(on_track_release)
                on(on_track_cancel)
            })
            .bind_field::<Node, _>(
                resource_changed::<EditorState>(),
                track_width,
                |node, width| {
                    node.width = width;
                    node.min_width = width;
                },
            )
            .with(|ui| {
                // The boxes get their own container so the watcher's
                // rebuild can't take the playhead with it.
                ui.bsn(bsn! {
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(TRACK_TOP_PADDING),
                        left: Val::Px(0.0),
                    }
                })
                .watch(value_changed(track_spans), build_track_boxes);

                ui.bsn(bsn! {
                    Playhead
                    playhead_line(0.0)
                })
                .bind_field::<Node, _>(
                    resource_changed::<MotionGfxManager>(),
                    current_time,
                    |node, time| {
                        node.left =
                            Val::Px(time * crate::PIXELS_PER_SECOND);
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

pub(crate) fn setup_editor_ui(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut registry: ResMut<WindowRegistry>,
    mut tree: ResMut<DockTree>,
    settings: Res<EditorSettings>,
) {
    let size = settings.physical_size.max(UVec2::ONE);
    let preview = images.add(Image::new_target_texture(
        size.x,
        size.y,
        TextureFormat::Rgba8Unorm,
        Some(TextureFormat::Rgba8UnormSrgb),
    ));
    commands.insert_resource(PreviewImage(preview.clone()));

    // Own render layer so this camera doesn't also pick up scene
    // meshes (e.g. bevy_vello's composite quad, layer 0)
    // full-window. `IsDefaultUiCamera` catches dock UI spawned
    // without a target (drag ghosts, drop overlays).
    let ui_camera = commands
        .spawn_scene(bsn! [
            Camera2d
            Camera {
                order: 10,
                clear_color: Color::BLACK,
            }
            TrackViewportCamera
        ])
        .insert((RenderLayers::layer(1), IsDefaultUiCamera))
        .id();

    if settings.hdr {
        commands.entity(ui_camera).insert(Hdr);
    }

    register_windows(&mut registry, preview);

    // Layout: viewport (+ settings tab) on top, timeline below.
    // Leaves are not persistent: emptied areas collapse
    // automatically.
    let viewport = tree.set_root_leaf(
        DockLeaf::new("viewport", DockAreaStyle::TabBar)
            .with_windows(vec![
                "viewport".into(),
                "hierarchy".into(),
                "settings".into(),
            ]),
    );
    tree.split(viewport, Edge::Bottom, "timeline".into());
    let split = tree.root.expect("root split exists");
    tree.set_fraction(split, 0.7);
    if let Some(timeline) = tree.find_leaf_with_window("timeline")
        && let Some(DockNode::Leaf(leaf)) = tree.get_mut(timeline)
    {
        leaf.area_id = "timeline".into();
    }

    // The kernel builds the whole tree under this root.
    commands.spawn((
        KernelRoot,
        UiTargetCamera(ui_camera),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
}

/// The app's UI tree. Everything reactive below here is a nested
/// `ui.watch` / `ui.bind`.
pub(crate) fn build_editor_ui(ui: &mut BevyUi) {
    // Non-visual binds live at the root: they hang off a node only for
    // lifetime, and write to resources or assets.
    crate::playback::bind_timeline_state(ui);
    bind_backdrop(ui);
    dock(ui);
}

/// Register the editor's dockable windows.
fn register_windows(
    registry: &mut WindowRegistry,
    preview: Handle<Image>,
) {
    registry.register(DockWindowDescriptor {
        id: "viewport".into(),
        name: "Viewport".into(),
        icon: None,
        build: Arc::new(move |ui: &mut BevyUi| {
            let preview = preview.clone();
            ui.bsn(bsn! {
                PreviewArea
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    min_height: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    overflow: Overflow::clip(),
                }
            })
            .with(move |ui| {
                ui.node(move |world, node| {
                    world.entity_mut(node).insert((
                        PreviewDisplay,
                        ImageNode::new(preview.clone()),
                    ));
                })
                // Letterboxed to fit the area above, which is this
                // node's parent.
                .bind_field::<Node, _>(
                    value_changed(crate::view::preview_fit),
                    crate::view::preview_fit,
                    |node, size| {
                        // `None` while the area has no size yet:
                        // leave the node alone rather than
                        // collapsing it to zero.
                        if let Some((width, height)) = size {
                            node.width = width;
                            node.height = height;
                        }
                    },
                );
            });
        }),
    });

    registry.register(DockWindowDescriptor {
        id: "timeline".into(),
        name: "Timeline".into(),
        icon: None,
        build: Arc::new(|ui: &mut BevyUi| timeline_panel(ui)),
    });

    registry.register(DockWindowDescriptor {
        id: "hierarchy".into(),
        name: "Hierarchy".into(),
        icon: None,
        build: Arc::new(|ui: &mut BevyUi| {
            crate::hierarchy::panel(ui)
        }),
    });

    // Settings: a reflect inspector over `EditorSettings` + Save.
    registry.register(DockWindowDescriptor {
        id: "settings".into(),
        name: "Settings".into(),
        icon: None,
        build: Arc::new(|ui: &mut BevyUi| {
            ui.bsn(bsn! {
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(PANEL_PADDING)),
                    overflow: Overflow::scroll_y(),
                }
                template_value(Glass::Panel)
            })
            .with(|ui| {
                // Editable rows built by the reflect inspector.
                inspector_fields::<EditorSettings>(ui);
                // Save row.
                ui.bsn(bsn! {
                    Node { flex_direction: FlexDirection::Row }
                    Children [(
                        glass_button()
                        on(|mut click: On<Pointer<Click>>,
                            mut commands: Commands| {
                            click.propagate(false);
                            commands.queue(SaveSettingsSync::Always);
                        })
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(24.0),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                        }
                        Children [
                            label::<SettingsSaveLabel>("Save")
                        ]
                    )]
                });
            });
        }),
    });
}

/// Drag handler for the name-panel / track resize divider.
pub(crate) fn on_divider_drag(
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

/// `timeline.target_time()`, or 0.0 if no timeline is focused yet.
fn current_time(world: &World, _: Entity) -> f32 {
    let state = world.resource::<EditorState>();
    let Some(id) = state.timeline else {
        return 0.0;
    };
    world
        .resource::<MotionGfxManager>()
        .get_timeline(&id)
        .map(|t| t.target_time())
        .unwrap_or(0.0)
}

/// Track node width for the current duration, floored at 1px so a
/// zero-duration track still lays out.
/// Marks one track's box in the timeline.
#[derive(Component, Default, Clone)]
pub(crate) struct TrackBox;

/// Every track's duration, in order. The watcher's signal: a box only
/// needs rebuilding when a track is added, removed or re-timed.
fn track_spans(world: &World, _: Entity) -> Vec<f32> {
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
    let spans = track_spans(ui.world(), ui.parent());
    let fill = ui.world().resource::<EditorTheme>().palette.blue;

    for (index, duration) in spans.into_iter().enumerate() {
        let top = index as f32 * (TRACK_HEIGHT + TRACK_GAP);
        let width = (duration * crate::PIXELS_PER_SECOND).max(1.0);
        ui.bsn(bsn! {
            TrackBox
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px({top}),
                left: Val::Px(0.0),
                width: Val::Px({width}),
                height: Val::Px(TRACK_HEIGHT),
                border_radius: BorderRadius::all(Val::Px(3.0)),
            }
            BackgroundColor({fill.with_alpha(0.35)})
        });
    }
}

fn track_width(world: &World, _: Entity) -> Val {
    let duration = world.resource::<EditorState>().duration;
    Val::Px((duration * crate::PIXELS_PER_SECOND).max(1.0))
}

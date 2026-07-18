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

use crate::playback::{TogglePlayback, on_scrub};
use crate::ui::dock::{
    DockAreaStyle, DockLeaf, DockNode, DockTree, DockTreeHost,
    DockWindowDescriptor, Edge, WindowRegistry,
};
use crate::ui::inspector::Inspector;
use crate::ui::glass::{Glass, glass_button};
use crate::ui::{Divider, label, playhead_line, scrub_slider};
use crate::{
    CONTROL_BAR_HEIGHT, EditorSettings, NAME_PANEL_MAX,
    NAME_PANEL_MIN, NAME_PANEL_WIDTH, PANEL_PADDING, PreviewImage,
    TRACK_TOP_PADDING,
};

/// Marker for the timeline panel node (fills its dock area).
#[derive(SceneComponent, Default, Clone)]
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

/// The [`ImageNode`] displaying the offscreen composition; sized to
/// fit [`PreviewArea`] by
/// [`fit_preview_image`](crate::view::fit_preview_image).
#[derive(Component, Default, Clone)]
pub(crate) struct PreviewDisplay;

/// Viewport where the timeline, track and action UI is displayed.
#[derive(SceneComponent, Default, Clone)]
pub(crate) struct TrackViewport;

/// The scrubbable track: a horizontal slider whose value is playback
/// time in seconds. Holds the action boxes and the playhead thumb.
///
/// The static skeleton is spawned by [`TrackViewport`]; its size,
/// time range and action boxes are filled in by
/// [`build_timeline_view`](crate::layout::build_timeline_view) once a
/// timeline exists.
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

impl EditorPanel {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                width: Val::Percent(100.0),
                // Fill the dock area; the dock split handle resizes it.
                flex_grow: 1.0,
                min_height: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::bottom(Val::Px(PANEL_PADDING)),
            }
            EditorPanel
            template_value(Glass::Panel)
            Children [
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
                                border_radius: BorderRadius::all(
                                    Val::Px(6.0),
                                ),
                            }
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
                            template_value(Glass::Panel)
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

impl TrackViewport {
    fn scene() -> impl Scene {
        bsn! {
            TrackViewport
            ScrollArea
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                // `min: 0` lets the viewport shrink below its (tall/wide)
                // content so it actually clips and scrolls.
                min_width: Val::Px(0.0),
                min_height: Val::Px(0.0),
                overflow: Overflow::scroll(),
            }
            template_value(Glass::Panel)
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
            .with_windows(vec!["viewport".into(), "settings".into()]),
    );
    tree.split(viewport, Edge::Bottom, "timeline".into());
    let split = tree.root.expect("root split exists");
    tree.set_fraction(split, 0.7);
    if let Some(timeline) = tree.find_leaf_with_window("timeline")
        && let Some(DockNode::Leaf(leaf)) = tree.get_mut(timeline)
    {
        leaf.area_id = "timeline".into();
    }

    // The dock reconciler fills this host with the tree's areas.
    commands.spawn((
        DockTreeHost,
        UiTargetCamera(ui_camera),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
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
        build: Arc::new(move |spawner| {
            let display = spawner
                .world_mut()
                .spawn((
                    PreviewDisplay,
                    ImageNode::new(preview.clone()),
                ))
                .id();
            spawner
                .spawn((
                    PreviewArea,
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        min_height: Val::Px(0.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .add_child(display);
        }),
    });

    registry.register(DockWindowDescriptor {
        id: "timeline".into(),
        name: "Timeline".into(),
        icon: None,
        build: Arc::new(|spawner| {
            spawner
                .spawn(())
                .apply_scene(bsn! { @EditorPanel })
                .expect("spawn timeline panel scene");
        }),
    });

    // Settings: a reflect inspector over `EditorSettings` + Save.
    registry.register(DockWindowDescriptor {
        id: "settings".into(),
        name: "Settings".into(),
        icon: None,
        build: Arc::new(|spawner| {
            let mut panel = spawner.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    padding: UiRect::all(Val::Px(PANEL_PADDING)),
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                Glass::Panel,
            ));
            let panel_id = panel.id();
            panel.world_scope(|world| {
                // Editable rows built by the reflect inspector.
                world.spawn((
                    Inspector::<EditorSettings>::default(),
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                    ChildOf(panel_id),
                ));
                // Save row.
                if let Ok(mut row) = world.spawn_scene(bsn! {
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
                }) {
                    row.insert(ChildOf(panel_id));
                }
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

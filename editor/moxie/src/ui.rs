//! The editor's dockable windows (viewport, timeline, hierarchy,
//! settings) and the startup system that spawns them against a
//! dedicated UI camera.

mod hierarchy;
mod timeline;

pub(crate) use timeline::TimelineContent;

use std::sync::Arc;

use bevy::camera::Hdr;
use bevy::camera::visibility::RenderLayers;
use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::settings::SaveSettingsSync;
use bevy::ui::widget::ImageNode;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};

use crate::{
    EditorSettings, EditorState, PreviewImage, playback, view,
};
use moxie_ui::MoxieUiPlugin;
use moxie_ui::elements::Label;
use moxie_ui::glass::{Glass, glass_button};
use moxie_ui::reactive::{
    BevyNodeMutExt, BevyUi, BevyUiExt, KernelSet, value_changed,
};
use moxie_ui::widgets::bind_backdrop;
use moxie_ui::widgets::dock::{
    DockAreaStyle, DockLeaf, DockNode, DockTree,
    DockWindowDescriptor, Edge, WindowRegistry, dock,
};
use moxie_ui::widgets::inspector::{
    InspectorTarget, inspector_fields,
};

/// Wires feathers theming, the editor UI tree, and the per-frame
/// timeline/playback/preview systems.
pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MoxieUiPlugin)
            .init_resource::<EditorState>()
            .add_systems(Startup, setup_editor_ui)
            .add_systems(
                Update,
                (
                    playback::play_pause_hotkey,
                    playback::stop_at_track_end,
                    view::retarget_scene_cameras,
                )
                    .chain()
                    .before(KernelSet),
            )
            .add_observer(playback::on_toggle_playback);
    }
}

pub(crate) const PANEL_PADDING: f32 = 12.0;

/// Marks the UI camera (which owns the window). Every other (scene)
/// camera is retargeted to the offscreen preview image; see
/// [`retarget_scene_cameras`].
///
/// [`retarget_scene_cameras`]: crate::view::retarget_scene_cameras
#[derive(Component, Default, Clone)]
pub(crate) struct TrackViewportCamera;

fn setup_editor_ui(
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

    // The kernel builds the whole tree under this root. `Commands`
    // can't reach `World` itself, so the build is queued: it runs
    // once these commands are applied, by which point `root` exists.
    let root = commands
        .spawn((
            UiTargetCamera(ui_camera),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.queue(move |world: &mut World| {
        moxie_ui::reactive::build_root(world, root, build_editor_ui);
    });
}

/// The app's UI tree. Everything reactive below here is a nested
/// `ui.watch` / `ui.bind`.
fn build_editor_ui(ui: &mut BevyUi) {
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
        icon: Some(crate::icons::VIEWPORT.into()),
        build: Arc::new(move |ui: &mut BevyUi| {
            let preview = preview.clone();
            ui.bsn(bsn! {
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
                ui.bundle(ImageNode::new(preview.clone()))
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
        icon: Some(crate::icons::TIMELINE.into()),
        build: Arc::new(|ui: &mut BevyUi| timeline::panel(ui)),
    });

    registry.register(DockWindowDescriptor {
        id: "hierarchy".into(),
        name: "Hierarchy".into(),
        icon: Some(crate::icons::HIERARCHY.into()),
        build: Arc::new(|ui: &mut BevyUi| hierarchy::panel(ui)),
    });

    // Settings: a reflect inspector over `EditorSettings` + Save.
    registry.register(DockWindowDescriptor {
        id: "settings".into(),
        name: "Settings".into(),
        icon: Some(crate::icons::SETTINGS.into()),
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
                inspector_fields(
                    ui,
                    InspectorTarget::resource::<EditorSettings>(),
                );
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
                        Children [(
                            @Label { @text: {"Save".to_string()} }
                        )]
                    )]
                });
            });
        }),
    });
}

mod action;
mod assets;
mod hierarchy;
mod inspector;
mod settings;
mod timeline;
mod top_bar;

use bevy::camera::Hdr;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs::schedule::common_conditions::resource_changed;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::ui::widget::ImageNode;
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};

use crate::{
    EditorSettings, EditorState, PreviewImage, ProjectBookmarks,
    ProjectPath, SelectedAction, SelectedEntity, playback, scene,
    view,
};
use bevy_fynix::EntityExt;
use fynix_mock::elem;
use moxie_ui::elements::{Frame, FrameCursor, Panel};
use moxie_ui::reactive::{BevyUi, FynixSet, value_changed};
use moxie_ui::widgets::dock::{
    DockAreaStyle, DockLeaf, DockNode, DockTree,
    DockWindowDescriptor, Edge, WindowRegistry, dock,
};
use moxie_ui::{MoxieUiPlugin, monokai};

/// Wires feathers theming, the editor UI tree, and the per-frame
/// timeline/playback/preview systems.
pub(crate) struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MoxieUiPlugin)
            .init_resource::<EditorState>()
            .init_resource::<SelectedAction>()
            .init_resource::<SelectedEntity>()
            .init_resource::<ProjectBookmarks>()
            .init_resource::<ProjectPath>()
            .init_resource::<assets::AssetFoldState>()
            .init_resource::<timeline::BlockFoldState>()
            .init_resource::<timeline::Dragging>()
            .init_resource::<hierarchy::Dragging>()
            .init_resource::<scene::EditorScene>()
            .add_systems(Startup, setup_editor_ui)
            .add_systems(
                Update,
                (
                    scene::recompile_dirty_scene.run_if(
                        resource_changed::<scene::EditorScene>,
                    ),
                    playback::track_first_timeline,
                    playback::play_pause_hotkey,
                    playback::stop_at_track_end,
                    view::retarget_scene_cameras,
                )
                    .chain()
                    .before(FynixSet),
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
                clear_color: { monokai::BASE[0] },
            }
            TrackViewportCamera
        ])
        .insert((RenderLayers::layer(1), IsDefaultUiCamera))
        .id();

    if settings.hdr {
        commands.entity(ui_camera).insert(Hdr);
    }

    register_windows(&mut registry);

    //
    // The dock layout.
    //
    let viewport = tree.set_root_leaf(
        DockLeaf::new("viewport", DockAreaStyle::TabBar)
            .with_windows(vec!["viewport".into()]),
    );

    tree.split(viewport, Edge::Bottom, "timeline".into());
    let vsplit = tree.root.expect("root split exists");
    tree.set_fraction(vsplit, 0.7);
    let timeline = tree
        .find_leaf_with_window("timeline")
        .expect("just split in a timeline leaf");
    if let Some(DockNode::Leaf(leaf)) = tree.get_mut(timeline) {
        leaf.area_id = "timeline".into();
    }

    tree.split(timeline, Edge::Right, "action".into());
    if let Some(hsplit) = tree.parent_of(timeline) {
        tree.set_fraction(hsplit, 0.8);
    }

    tree.split(viewport, Edge::Right, "inspector".into());
    if let Some(hsplit) = tree.parent_of(viewport) {
        tree.set_fraction(hsplit, 0.8);
    }

    if let Some((sidebar, hierarchy_tab)) =
        tree.split(viewport, Edge::Left, "hierarchy".into())
    {
        // `add_tab` activates what it just added; Hierarchy stays the
        // one shown on a fresh layout.
        tree.add_tab(sidebar, "assets");
        tree.set_active(sidebar, hierarchy_tab);
    }
    if let Some(hsplit) = tree.parent_of(viewport) {
        tree.set_fraction(hsplit, 0.2);
    }

    // The kernel builds the whole tree under this root. `Commands`
    // can't reach `World` itself, so the build is queued: it runs
    // once these commands are applied, by which point `root` exists.
    let root = commands
        .spawn((
            UiTargetCamera(ui_camera),
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.queue(move |world: &mut World| {
        moxie_ui::reactive::watch_root(world, root, build_editor_ui);
    });
}

/// The app's UI tree. Everything reactive below here is a nested
/// `ui.watch` / `ui.bind`.
fn build_editor_ui(ui: &mut BevyUi) {
    // Non-visual binds live at the root: they hang off a node only for
    // lifetime, and write to resources or assets.
    ui.compose(top_bar::TopBar);
    dock(ui);
}

/// Register the editor's dockable windows.
fn register_windows(registry: &mut WindowRegistry) {
    registry.register(DockWindowDescriptor {
        id: "viewport".into(),
        name: "Viewport".into(),
        icon: Some(crate::icons::VIEWPORT.into()),
        build: |ui: &mut BevyUi| {
            let preview =
                ui.world.resource::<PreviewImage>().0.clone();
            ui.elem(elem!(
                Panel,
                justify = JustifyContent::Center,
                align = AlignItems::Center
            ))
            .with(move |ui| {
                let fit =
                    crate::view::preview_fit(ui.world, ui.parent());
                let shown = fit.is_some();

                // Letterboxed to fit the area above. Hidden until that
                // area has a size: at a fresh `ComputedNode` it does
                // not, and `Auto` would flash at the image's native
                // size for a frame.
                ui.elem(elem!(
                    Frame,
                    width = fit.map_or(Val::ZERO, |(width, _)| width),
                    height =
                        fit.map_or(Val::ZERO, |(_, height)| height),
                    display = if shown {
                        Display::Flex
                    } else {
                        Display::None
                    }
                ))
                .insert(ImageNode::new(preview.clone()))
                .bind(
                    |frame| frame.width(),
                    value_changed(crate::view::preview_fit),
                    |world, node| {
                        crate::view::preview_fit(world, node)
                            .map_or(Val::ZERO, |(width, _)| width)
                    },
                )
                .bind(
                    |frame| frame.height(),
                    value_changed(crate::view::preview_fit),
                    |world, node| {
                        crate::view::preview_fit(world, node)
                            .map_or(Val::ZERO, |(_, height)| height)
                    },
                )
                .bind(
                    |frame| frame.display(),
                    value_changed(crate::view::preview_fit),
                    |world, node| {
                        if crate::view::preview_fit(world, node)
                            .is_some()
                        {
                            Display::Flex
                        } else {
                            Display::None
                        }
                    },
                );
            });
        },
    });

    registry.register(DockWindowDescriptor {
        id: "timeline".into(),
        name: "Timeline".into(),
        icon: Some(crate::icons::TIMELINE.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(timeline::TimelinePanel);
        },
    });

    registry.register(DockWindowDescriptor {
        id: "hierarchy".into(),
        name: "Hierarchy".into(),
        icon: Some(crate::icons::HIERARCHY.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(hierarchy::HierarchyPanel);
        },
    });

    registry.register(DockWindowDescriptor {
        id: "action".into(),
        name: "Action".into(),
        icon: Some(crate::icons::ACTION.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(action::ActionPanel);
        },
    });

    registry.register(DockWindowDescriptor {
        id: "inspector".into(),
        name: "Inspector".into(),
        icon: Some(crate::icons::INSPECTOR.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(inspector::InspectorPanel);
        },
    });

    // Settings: a reflect inspector over `EditorSettings` + Save.
    registry.register(DockWindowDescriptor {
        id: "settings".into(),
        name: "Settings".into(),
        icon: Some(crate::icons::SETTINGS.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(settings::SettingsPanel);
        },
    });

    registry.register(DockWindowDescriptor {
        id: "assets".into(),
        name: "Assets".into(),
        icon: Some(crate::icons::ASSETS.into()),
        build: |ui: &mut BevyUi| {
            ui.compose(assets::AssetsPanel);
        },
    });
}

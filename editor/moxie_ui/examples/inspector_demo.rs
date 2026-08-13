//! Demonstrates the reflection-driven inspector in
//! [`moxie_ui::widgets::inspector`]: primitive widgets, the compact
//! per-axis row every float/signed/unsigned glam vector gets instead
//! of folding its components away, and the collapsible group a plain
//! nested struct earns for free by having no [`Inspect`](
//! moxie_ui::widgets::inspector::Inspect) of its own.
//!
//! Edit any field; nothing here reacts to the values, so it's purely
//! a look at the widgets themselves. Click a group's header ("Local
//! Transform") to fold and unfold it.

use bevy::prelude::*;
use fynix_mock::elem;
use moxie_ui::MoxieUiPlugin;
use moxie_ui::fynix::{Frame, Label};
use moxie_ui::reactive::BevyUi;
use moxie_ui::theme::EditorTheme;
use moxie_ui::widgets::inspector::{
    InspectorTarget, inspector_fields,
};

fn main() {
    App::new()
        .add_plugins((
            // `../assets`: the editor crates share one asset folder
            // (`editor/assets`) rather than each carrying its own.
            DefaultPlugins.set(AssetPlugin {
                file_path: "../assets".into(),
                ..default()
            }),
            MoxieUiPlugin,
        ))
        .register_type::<Showcase>()
        .register_type::<LocalTransform>()
        .insert_resource(Showcase::default())
        .add_systems(Startup, setup)
        .run();
}

/// Every widget the default registrations cover, plus a nested struct
/// with none of its own to show the fold.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource, Default)]
struct Showcase {
    visible: bool,
    brightness: f32,
    samples: u32,
    tint: Vec4,
    uv_offset: Vec2,
    grid_size: IVec2,
    texture_size: UVec2,
    transform: Transform,
    local: LocalTransform,
}

/// No [`Inspect`](moxie_ui::widgets::inspector::Inspect) impl of its
/// own, so the inspector shows it as a collapsible group rather than
/// flattening its fields into `Showcase`'s own list.
#[derive(Reflect, Default)]
struct LocalTransform {
    translation: Vec3,
    scale: Vec3,
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // The kernel builds the panel under this full-window root. Queued:
    // `Commands` can't reach `World` itself, so this runs once these
    // commands are applied, by which point `root` exists.
    let root = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(px(24)),
            ..default()
        })
        .id();
    commands.queue(move |world: &mut World| {
        moxie_ui::reactive::watch_root(world, root, build_panel);
    });
}

fn build_panel(ui: &mut BevyUi) {
    let theme = ui.world.resource::<EditorTheme>().clone();

    ui.elem(elem!(
        Frame,
        width = px(360),
        direction = FlexDirection::Column,
        row_gap = px(12),
        padding = UiRect::all(px(12)),
        radius = px(8),
        background = theme.palette.base[1]
    ))
    .with(move |ui| {
        ui.elem(elem!(
            Label,
            text = "Showcase",
            size = 14.0f32,
            bold = true,
            color = Some(theme.text_primary)
        ));
        inspector_fields(ui, InspectorTarget::resource::<Showcase>());
    });
}

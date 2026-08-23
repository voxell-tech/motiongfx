//! Demonstrates the reflection-driven inspector in
//! [`moxie_ui::inspector`], through the three composers that
//! mount one: a resource, one component of an entity, and every
//! component of an entity at once.
//!
//! Along the way it shows what the field walk does with what it
//! finds: a widget per primitive, the compact per-axis row every
//! float/signed/unsigned glam vector gets instead of folding its
//! components away, and the collapsible group a plain nested struct
//! earns for free by having no [`Inspect`](
//! moxie_ui::inspector::Inspect) of its own.
//!
//! Edit any field; nothing here reacts to the values, so it's purely
//! a look at the widgets themselves. Click any header to fold it.

use bevy::prelude::*;
use fynix_mock::elem;
use moxie_ui::MoxieUiPlugin;
use moxie_ui::elements::{
    ComponentInspector, EntityInspector, Frame, Label,
    ResourceInspector,
};
use moxie_ui::inspector::InspectAppExt;
use moxie_ui::reactive::BevyUi;
use moxie_ui::theme::EditorTheme;

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
        .register_inspectable::<Orbit>()
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
    local: LocalTransform,
}

/// No [`Inspect`](moxie_ui::inspector::Inspect) impl of its
/// own, so the inspector shows it as a collapsible group rather than
/// flattening its fields into `Showcase`'s own list.
#[derive(Reflect, Default)]
struct LocalTransform {
    translation: Vec3,
    scale: Vec3,
}

/// A component of the demo's own, so the entity has something on it
/// besides what bevy puts there.
#[derive(Component, Reflect, Default)]
#[reflect(Component, Default)]
struct Orbit {
    radius: f32,
    speed: f32,
}

/// The entity the component and entity inspectors are pointed at.
#[derive(Resource)]
struct Subject(Entity);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // `Transform` also drags `GlobalTransform` in, but the entity
    // inspector only shows what's registered `register_inspectable` -
    // `GlobalTransform` is reflected, never opted in, so it stays out.
    let subject = commands
        .spawn((
            Transform::from_xyz(1.0, 2.0, 3.0),
            Orbit {
                radius: 4.0,
                speed: 0.5,
            },
        ))
        .id();
    commands.insert_resource(Subject(subject));

    // The kernel builds the panels under this full-window root.
    // Queued: `Commands` can't reach `World` itself, so this runs
    // once these commands are applied, by which point `root` and
    // `Subject` both exist.
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
        moxie_ui::reactive::watch_root(world, root, build_panels);
    });
}

fn build_panels(ui: &mut BevyUi) {
    let theme = ui.theme;
    let subject = ui.world.resource::<Subject>().0;

    ui.elem(elem!(
        Frame,
        direction = FlexDirection::Row,
        align = AlignItems::FlexStart,
        column_gap = px(16)
    ))
    .with(move |ui| {
        panel(ui, theme, "Resource", |ui| {
            ui.compose(ResourceInspector::of::<Showcase>());
        });
        panel(ui, theme, "Component", move |ui| {
            ui.compose(ComponentInspector::of::<Transform>(subject));
        });
        panel(ui, theme, "Entity", move |ui| {
            ui.compose(EntityInspector { entity: subject });
        });
    });
}

/// A titled card, which is all these three have in common.
fn panel(
    ui: &mut BevyUi,
    theme: &EditorTheme,
    title: &str,
    body: impl FnOnce(&mut BevyUi),
) {
    let title = title.to_string();
    let text_color = theme.text_primary;

    ui.elem(elem!(
        Frame,
        direction = FlexDirection::Column,
        row_gap = px(12),
        padding = UiRect::all(px(12)),
        radius = px(8),
        background = theme.palette.base[1]
    ))
    .with(move |ui| {
        ui.elem(elem!(
            Label,
            text = title,
            size = 14.0f32,
            bold = true,
            color = Some(text_color)
        ));
        body(ui);
    });
}

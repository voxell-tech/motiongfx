//! Demonstrates the standalone docking system in
//! [`moxie_ui::widgets::dock`].
//!
//! Three trivial panels ("Panel A/B/C") start as tabs in one full-window
//! area. Try:
//! - dragging a tab left/right within the tab bar to reorder it,
//! - dragging a tab onto another area's tab bar to merge it in,
//! - dragging a tab onto an area's top/bottom/left/right edge to split,
//! - dragging the divider between two areas to resize them,
//! - pressing Escape mid-drag to cancel.

use bevy::prelude::*;
use fynix_mock::elem;
use moxie_ui::MoxieUiPlugin;
use moxie_ui::elements::{Frame, Label};
use moxie_ui::reactive::BevyUi;
use moxie_ui::widgets::dock::{
    DockAreaStyle, DockLeaf, DockTree, DockWindowBuildFn,
    DockWindowDescriptor, WindowRegistry, dock,
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
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut registry: ResMut<WindowRegistry>,
    mut tree: ResMut<DockTree>,
) {
    commands.spawn(Camera2d);

    // Register three trivial window kinds, each filling its content
    // area with a colored label. A builder is a bare `fn`, so what a
    // panel shows cannot be captured at registration - hence one
    // function per kind.
    for (id, name, build) in [
        ("panel_a", "Panel A", panel_a as DockWindowBuildFn),
        ("panel_b", "Panel B", panel_b),
        ("panel_c", "Panel C", panel_c),
    ] {
        registry.register(DockWindowDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            icon: None,
            build,
        });
    }

    // Seed the layout: one root leaf holding all three panels as tabs.
    tree.set_root_leaf(
        DockLeaf::new("root", DockAreaStyle::TabBar).with_windows(
            vec![
                "panel_a".into(),
                "panel_b".into(),
                "panel_c".into(),
            ],
        ),
    );

    // The kernel builds the dock under this full-window root. Queued:
    // `Commands` can't reach `World` itself, so this runs once these
    // commands are applied, by which point `root` exists.
    let root = commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: percent(100),
            height: percent(100),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    commands.queue(move |world: &mut World| {
        moxie_ui::reactive::watch_root(world, root, dock);
    });
}

fn panel_a(ui: &mut BevyUi) {
    filled(ui, "Panel A", Color::srgb(0.20, 0.28, 0.40));
}

fn panel_b(ui: &mut BevyUi) {
    filled(ui, "Panel B", Color::srgb(0.28, 0.20, 0.34));
}

fn panel_c(ui: &mut BevyUi) {
    filled(ui, "Panel C", Color::srgb(0.20, 0.34, 0.26));
}

/// A panel's whole content: its name, centred on its own colour.
fn filled(ui: &mut BevyUi, label: &str, color: Color) {
    let label = label.to_string();

    ui.elem(elem!(
        Frame,
        width = percent(100),
        height = percent(100),
        align = AlignItems::Center,
        justify = JustifyContent::Center,
        background = color
    ))
    .with(move |ui| {
        ui.elem(elem!(
            Label,
            text = label,
            size = 20.0f32,
            color = Some(Color::srgb(0.9, 0.9, 0.92))
        ));
    });
}

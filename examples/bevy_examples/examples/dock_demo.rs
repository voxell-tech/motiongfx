//! Demonstrates the standalone docking system in
//! [`motiongfx_editor_ui::dock`].
//!
//! Three trivial panels ("Panel A/B/C") start as tabs in one full-window
//! area. Try:
//! - dragging a tab left/right within the tab bar to reorder it,
//! - dragging a tab onto another area's tab bar to merge it in,
//! - dragging a tab onto an area's top/bottom/left/right edge to split,
//! - dragging the divider between two areas to resize them,
//! - pressing Escape mid-drag to cancel.

use std::sync::Arc;

use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;
use motiongfx_editor_ui::dock::{
    DockAreaStyle, DockLeaf, DockPlugin, DockTree,
    DockWindowDescriptor, WindowRegistry, dock,
};
use motiongfx_editor_ui::reactive::{
    BevyUi, BevyUiExt, KernelPlugin, KernelRoot,
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FeathersPlugins,
            DockPlugin,
            KernelPlugin::new(dock),
        ))
        // Seed the feathers palette (its default theme is empty).
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut registry: ResMut<WindowRegistry>,
    mut tree: ResMut<DockTree>,
) {
    commands.spawn(Camera2d);

    // Register three trivial window kinds. Each just fills its content
    // area with a colored label.
    for (id, name, color) in [
        ("panel_a", "Panel A", Color::srgb(0.20, 0.28, 0.40)),
        ("panel_b", "Panel B", Color::srgb(0.28, 0.20, 0.34)),
        ("panel_c", "Panel C", Color::srgb(0.20, 0.34, 0.26)),
    ] {
        let label = name.to_string();
        registry.register(DockWindowDescriptor {
            id: id.to_string(),
            name: name.to_string(),
            icon: None,
            build: Arc::new(move |ui: &mut BevyUi| {
                let label = label.clone();
                ui.bsn(bsn! {
                    Node {
                        flex_grow: 1.0,
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                    }
                    BackgroundColor({color})
                })
                .with(move |ui| {
                    ui.bsn(bsn! {
                        Text({label})
                        TextFont { font_size: FontSize::Px(20.0) }
                        TextColor(Color::srgb(0.9, 0.9, 0.92))
                    });
                });
            }),
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

    // The kernel builds the dock under this full-window root.
    commands.spawn((
        KernelRoot,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
}

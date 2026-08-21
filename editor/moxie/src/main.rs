//! The editor binary: [`MoxiePlugin`] over an empty scene.
//!
//! Nothing is spawned here beyond a camera and a light - `File > Open`
//! is how a project actually gets its content.

use bevy::asset::UnapprovedPathMode;
use bevy::{prelude::*, window::WindowResolution};
use bevy_motiongfx::BevyMotionGfxPlugin;
use moxie::MoxiePlugin;

fn main() {
    let mut app = App::new();

    // Before `DefaultPlugins`: asset sources build when `AssetPlugin`
    // does, not after.
    moxie_ui::asset::register_absolute_source(&mut app);

    app.add_plugins((
        // `../assets`: the editor crates share one asset folder
        // (`editor/assets`) rather than each carrying its own.
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "../assets".into(),
                // A dragged file's own path is outside this root by
                // construction, so it needs the per-load override
                // `Deny` allows; the stricter default (`Forbid`)
                // has no such escape hatch.
                unapproved_path_mode: UnapprovedPathMode::Deny,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920, 1080),
                    ..default()
                }),
                ..default()
            }),
        BevyMotionGfxPlugin,
        MoxiePlugin,
    ))
    .add_systems(Startup, setup)
    .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera {
            clear_color: Color::srgb(0.02, 0.02, 0.04).into(),
            ..default()
        },
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 14.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 10.0, 5.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

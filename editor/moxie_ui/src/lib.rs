//! Reusable UI elements and widgets for Moxie.

#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Inherent to Bevy ECS: systems take many params and query tuples."
)]

pub mod elements;
pub mod fold;
pub mod icons;
pub mod inspector;
pub mod layout;
pub mod motion;
pub mod palette;
pub mod reactive;
pub mod theme;
pub mod widgets;

use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;

use inspector::InspectPlugin;
use reactive::FynixPlugin;
use theme::EditorTheme;
use widgets::dock::DockPlugin;

/// Everything a consumer needs to render a moxie UI: feathers theming,
/// the dock engine, the
/// default reflect-inspector widgets, and the kernel.
///
/// Doesn't build a root itself; spawn one and call
/// [`reactive::watch_root`] wherever the app does its own `Startup`
/// setup.
#[derive(Default)]
pub struct MoxieUiPlugin;

impl Plugin for MoxieUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FeathersPlugins,
            DockPlugin,
            FynixPlugin,
            InspectPlugin,
        ))
        // The colours every widget reads.
        .init_resource::<EditorTheme>()
        // Seed the feathers palette (its default theme is empty).
        .insert_resource(UiTheme(create_dark_theme()));
    }
}

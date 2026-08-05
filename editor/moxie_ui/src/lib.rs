//! Reusable UI elements and widgets for Moxie.

#![allow(
    clippy::type_complexity,
    clippy::too_many_arguments,
    reason = "Inherent to Bevy ECS: systems take many params and query tuples."
)]

pub mod elements;
pub mod glass;
pub mod icons;
pub mod reactive;
pub mod theme;
pub mod widgets;

use bevy::feathers::FeathersPlugins;
use bevy::feathers::dark_theme::create_dark_theme;
use bevy::feathers::theme::UiTheme;
use bevy::prelude::*;

use glass::GlassPlugin;
use reactive::KernelPlugin;
use widgets::dock::DockPlugin;
use widgets::inspector::InspectAppExt;

/// Everything a consumer needs to render a moxie UI: feathers theming,
/// the dock engine (which pulls in [`glass::GlassPlugin`]), the
/// default reflect-inspector widgets, and the kernel.
///
/// Doesn't build a root itself — spawn one and call
/// [`reactive::build_root`] wherever the app does its own `Startup`
/// setup.
#[derive(Default)]
pub struct MoxieUiPlugin;

impl Plugin for MoxieUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            FeathersPlugins,
            DockPlugin,
            KernelPlugin,
            GlassPlugin,
        ))
        .register_default_inspects()
        // Seed the feathers palette (its default theme is empty).
        .insert_resource(UiTheme(create_dark_theme()));
    }
}

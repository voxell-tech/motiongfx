//! Frosted-glass `UiMaterial`: a translucent tinted body that blurs the
//! backdrop behind it, with a thin border rim and a cursor glow on
//! hovered interactables.
//!
//! Usage: insert a [`Glass`] preset marker on a node
//! (`template_value(Glass::Panel)`), or spawn a widget builder
//! ([`glass_button`], [`glass_checkbox`], [`glass_number_field`]); an
//! observer swaps in the matching material. Don't also set
//! `BackgroundColor`/`BorderColor` — the material replaces both.
//! Corner rounding comes from the node's own `BorderRadius`.

mod backdrop;
mod glow;
mod material;
mod preset;
mod widget;

pub use backdrop::{GlassBackdrop, bind_backdrop};
use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::ui_render::prelude::UiMaterialPlugin;
pub use material::GlassMaterial;
pub use preset::{Glass, GlassAssets};
pub use widget::{glass_button, glass_checkbox, glass_number_field};

use crate::theme::EditorTheme;

pub struct GlassPlugin;

impl Plugin for GlassPlugin {
    fn build(&self, app: &mut App) {
        // Keep the embedded shader path (`glass.wgsl`) resolving
        // against this file; `GlassMaterial::fragment_shader`
        // matches.
        embedded_asset!(app, "glass.wgsl");
        app.init_resource::<EditorTheme>()
            .add_plugins(UiMaterialPlugin::<GlassMaterial>::default())
            .add_observer(preset::attach_glass)
            .add_systems(
                Update,
                (
                    glow::update_glass_buttons,
                    glow::update_glow,
                    widget::update_glass_checkmarks,
                    widget::update_glass_field_cursors,
                ),
            );

        let theme = app.world().resource::<EditorTheme>().clone();
        let mut materials =
            app.world_mut().resource_mut::<Assets<GlassMaterial>>();
        let assets = preset::build_assets(&theme, &mut materials);
        app.insert_resource(assets);
    }
}

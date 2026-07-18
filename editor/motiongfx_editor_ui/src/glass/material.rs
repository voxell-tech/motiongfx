//! The frosted-glass [`UiMaterial`] and its builder.

use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui_render::prelude::UiMaterial;

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct GlassMaterial {
    /// Base glass tint; alpha is the body opacity.
    #[uniform(0)]
    pub tint: LinearRgba,
    /// x: rim brightness, y: frost blur radius (physical px),
    /// z: rim opacity, w: cursor glow strength.
    #[uniform(1)]
    pub params: Vec4,
    /// xy: cursor position (physical px), z: glow radius (physical
    /// px), w: per-material radius scale (smaller = tighter glow).
    /// xyz written every frame by `update_glow`; w is preserved.
    #[uniform(2)]
    pub glow: Vec4,
    /// Backdrop rect in physical px (min.xy, size.zw); zero disables
    /// frost. Written by `sync_backdrop`.
    #[uniform(3)]
    pub backdrop_rect: Vec4,
    /// The backdrop image (e.g. wallpaper) frosted behind panes.
    #[texture(4)]
    #[sampler(5)]
    pub backdrop: Option<Handle<Image>>,
    /// x: edge feather in px (0 = crisp; larger = soft frosted
    /// silhouette fading inward).
    #[uniform(6)]
    pub extra: Vec4,
    /// Configured glow strength; copied into `params.w` only while
    /// an entity using this material is hovered (CPU-side, not
    /// bound).
    pub base_glow: f32,
}

impl GlassMaterial {
    /// `frost` is the blur radius in physical px (0 disables).
    pub fn new(tint: LinearRgba, rim: f32, frost: f32) -> Self {
        Self {
            tint,
            // Defaults: near-opaque thin rim, no cursor glow; only
            // interactable surfaces opt in via `glow_strength`.
            params: Vec4::new(rim, frost, 0.85, 0.0),
            glow: Vec4::new(f32::MIN, f32::MIN, 1.0, 1.0),
            backdrop_rect: Vec4::ZERO,
            backdrop: None,
            extra: Vec4::ZERO,
            base_glow: 0.0,
        }
    }

    /// Soften the pane's silhouette: alpha fades inward over `px`.
    pub fn feather(mut self, px: f32) -> Self {
        self.extra.x = px;
        self
    }

    /// Opacity of the thin border rim (the pane's dense edge).
    pub fn rim_opacity(mut self, opacity: f32) -> Self {
        self.params.z = opacity;
        self
    }

    /// How strongly this surface picks up the cursor glow while one
    /// of its users is hovered.
    pub fn glow_strength(mut self, strength: f32) -> Self {
        self.base_glow = strength;
        self
    }

    /// Scale of the glow radius for this surface (1.0 = shared
    /// default; smaller = tighter, more concentrated glow).
    pub fn glow_radius_scale(mut self, scale: f32) -> Self {
        self.glow.w = scale;
        self
    }
}

impl UiMaterial for GlassMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://motiongfx_editor_ui/glass.wgsl".into()
    }
}

//! Frosted-glass [`UiMaterial`]: a translucent tinted body that blurs
//! the backdrop image behind it, with a thin near-opaque border rim
//! and a cursor glow on hovered interactables. Corner rounding comes
//! from the node's own `BorderRadius`.
//!
//! Usage: add a [`Glass`] preset to any node (works in `bsn!` and
//! world spawns); an observer attaches the matching
//! [`MaterialNode<GlassMaterial>`]. Don't combine with
//! `BackgroundColor`/`BorderColor` — the material replaces both.
//! [`glass_button`] is a drop-in glass-styled headless button.

use bevy::asset::embedded_asset;
use bevy::feathers::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::ui::Pressed;
use bevy::ui_render::prelude::{MaterialNode, UiMaterial, UiMaterialPlugin};
use bevy::ui_widgets::Button;
use bevy::window::SystemCursorIcon;

use super::theme::EditorTheme;

pub struct GlassPlugin;

impl Plugin for GlassPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "glass.wgsl");
        app.init_resource::<EditorTheme>()
            .add_plugins(UiMaterialPlugin::<GlassMaterial>::default())
            .add_observer(attach_glass)
            .add_systems(
                Update,
                (update_glass_buttons, update_glow, sync_backdrop),
            );

        // Derive every preset from the theme palette.
        let theme = app.world().resource::<EditorTheme>().clone();
        let tint = |color: Color, alpha: f32| -> LinearRgba {
            color.with_alpha(alpha).to_linear()
        };
        let base = &theme.palette.base;
        let accent = theme.accent;

        let mut materials = app
            .world_mut()
            .resource_mut::<Assets<GlassMaterial>>();
        // `new(tint, rim, frost_px)`: frost is the backdrop blur.
        let assets = GlassAssets {
            panel: materials.add(
                GlassMaterial::new(tint(base[2], 0.30), 0.10, 6.0)
                    .rim_opacity(0.60),
            ),
            bar: materials.add(GlassMaterial::new(
                tint(base[3], 0.35),
                0.22,
                8.0,
            )),
            // Popup stays dense for text.
            popup: materials.add(
                GlassMaterial::new(tint(base[2], 0.85), 0.35, 10.0)
                    .rim_opacity(0.95),
            ),
            ghost: materials.add(
                GlassMaterial::new(tint(base[3], 0.55), 0.40, 8.0)
                    .rim_opacity(0.95)
                    .feather(3.0),
            ),
            // Soft frosted drop hint.
            overlay: materials.add(
                GlassMaterial::new(tint(accent, 0.20), 0.0, 4.0)
                    .rim_opacity(0.0)
                    .feather(6.0),
            ),
            // Tabs are interactable: a tight glow signals it, even on
            // otherwise-invisible idle tabs.
            tab_idle: materials.add(
                GlassMaterial::new(LinearRgba::NONE, 0.0, 0.0)
                    .rim_opacity(0.0)
                    .glow_strength(1.20)
                    .glow_radius_scale(0.35),
            ),
            // Faint neutral pill under the cursor.
            tab_hover: materials.add(
                GlassMaterial::new(tint(base[5], 0.15), 0.18, 5.0)
                    .rim_opacity(0.40)
                    .glow_strength(1.20)
                    .glow_radius_scale(0.35),
            ),
            // Dim resting pill so the hover glow reads clearly.
            tab_active: materials.add(
                GlassMaterial::new(tint(accent, 0.10), 0.25, 6.0)
                    .rim_opacity(0.55)
                    .glow_strength(1.30)
                    .glow_radius_scale(0.35),
            ),
            button: materials.add(
                GlassMaterial::new(tint(base[4], 0.30), 0.28, 5.0)
                    .glow_strength(1.30)
                    .glow_radius_scale(0.35),
            ),
            button_hover: materials.add(
                GlassMaterial::new(tint(base[5], 0.40), 0.38, 5.0)
                    .glow_strength(1.60)
                    .glow_radius_scale(0.35),
            ),
            button_pressed: materials.add(
                GlassMaterial::new(tint(base[3], 0.50), 0.18, 5.0)
                    .glow_strength(1.00)
                    .glow_radius_scale(0.35),
            ),
        };
        app.insert_resource(assets);
    }
}

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
    /// xyz written every frame by [`update_glow`]; w is preserved.
    #[uniform(2)]
    pub glow: Vec4,
    /// Backdrop rect in physical px (min.xy, size.zw); zero disables
    /// frost. Written by [`sync_backdrop`].
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
    /// Configured glow strength; copied into `params.w` only while an
    /// entity using this material is hovered (CPU-side, not bound).
    pub base_glow: f32,
}

impl GlassMaterial {
    /// `frost` is the blur radius in physical px (0 disables).
    pub fn new(tint: LinearRgba, rim: f32, frost: f32) -> Self {
        Self {
            tint,
            // Defaults: near-opaque thin rim, no cursor glow — only
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
        "embedded://motiongfx_editor/ui/glass.wgsl".into()
    }
}

/// Declarative glass preset. Inserting one (or re-inserting a
/// different variant) swaps the node's material accordingly.
#[derive(Component, Clone, Copy, Default, PartialEq, Eq)]
pub enum Glass {
    /// Large content surfaces: barely-there tint.
    #[default]
    Panel,
    /// Tab bars and similar chrome strips.
    Bar,
    /// Floating menus: opaque enough for text.
    Popup,
    /// Drag ghost card.
    Ghost,
    /// Drop-target feedback rects.
    Overlay,
    /// Inactive tab: fully invisible (keeps the slot uniform).
    TabIdle,
    /// Hovered inactive tab: faint pill so the target reads.
    TabHover,
    /// Active tab pill.
    TabActive,
    /// Buttons (idle; hover/press handled by [`update_glass_buttons`]).
    Button,
}

/// Shared handles for every [`Glass`] preset.
#[derive(Resource)]
pub struct GlassAssets {
    pub panel: Handle<GlassMaterial>,
    pub bar: Handle<GlassMaterial>,
    pub popup: Handle<GlassMaterial>,
    pub ghost: Handle<GlassMaterial>,
    pub overlay: Handle<GlassMaterial>,
    pub tab_idle: Handle<GlassMaterial>,
    pub tab_hover: Handle<GlassMaterial>,
    pub tab_active: Handle<GlassMaterial>,
    pub button: Handle<GlassMaterial>,
    pub button_hover: Handle<GlassMaterial>,
    pub button_pressed: Handle<GlassMaterial>,
}

impl GlassAssets {
    fn preset(&self, glass: Glass) -> Handle<GlassMaterial> {
        match glass {
            Glass::Panel => self.panel.clone(),
            Glass::Bar => self.bar.clone(),
            Glass::Popup => self.popup.clone(),
            Glass::Ghost => self.ghost.clone(),
            Glass::Overlay => self.overlay.clone(),
            Glass::TabIdle => self.tab_idle.clone(),
            Glass::TabHover => self.tab_hover.clone(),
            Glass::TabActive => self.tab_active.clone(),
            Glass::Button => self.button.clone(),
        }
    }
}

/// A glass-styled button carrying the headless [`Button`] behavior;
/// drop-in replacement for `themed_button`. Emits
/// [`bevy::ui_widgets::Activate`].
pub fn glass_button<M>(width: f32, height: f32) -> impl Scene
where
    M: Component + Default + Unpin + Clone,
{
    bsn! {
        M
        template_value(Glass::Button)
        Button
        Hovered
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
        }
        EntityCursor::System(SystemCursorIcon::Pointer)
    }
}

/// Attach / swap the material whenever a [`Glass`] preset is inserted.
fn attach_glass(
    insert: On<Insert, Glass>,
    q_glass: Query<&Glass>,
    assets: Res<GlassAssets>,
    mut commands: Commands,
) {
    let Ok(glass) = q_glass.get(insert.entity) else {
        return;
    };
    commands
        .entity(insert.entity)
        .insert(MaterialNode(assets.preset(*glass)));
}

/// Marks a UI node displaying a backdrop image (e.g. the editor's
/// scene preview) that glass panes refract where they overlap it.
/// Carries the image so the glass module stays app-agnostic.
#[derive(Component)]
pub struct GlassBackdrop(pub Handle<Image>);

/// Mirror the backdrop node's on-screen rect (physical px, matching
/// the fragment shader's framebuffer coordinates) and image into
/// every glass material.
fn sync_backdrop(
    q_backdrop: Query<(
        &GlassBackdrop,
        &ComputedNode,
        &bevy::ui::UiGlobalTransform,
    )>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut last: Local<Vec4>,
) {
    let source = q_backdrop.single().ok();
    let rect = source.map_or(Vec4::ZERO, |(_, computed, transform)| {
        let size = computed.size();
        let (_, _, center) = transform.to_scale_angle_translation();
        let min = center.trunc() - size * 0.5;
        Vec4::new(min.x, min.y, size.x, size.y)
    });
    if *last == rect {
        return;
    }
    *last = rect;

    let image = source.map(|(backdrop, _, _)| backdrop.0.clone());
    for (_, material) in materials.iter_mut() {
        material.backdrop_rect = rect;
        material.backdrop = image.clone();
    }
}

/// Drive the cursor glow: push the cursor position (physical pixels,
/// matching the fragment shader's framebuffer coordinates) into every
/// glass material, and enable each material's glow only while one of
/// the entities using it is actually hovered.
fn update_glow(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    q_surfaces: Query<(
        &MaterialNode<GlassMaterial>,
        Option<&Hovered>,
        Option<&Interaction>,
    )>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut last: Local<(Vec2, HashSet<AssetId<GlassMaterial>>)>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    // Park the glow far away when the cursor leaves the window.
    let pos = window
        .cursor_position()
        .map(|p| p * window.scale_factor())
        .unwrap_or(Vec2::splat(-1.0e6));

    // Materials with at least one hovered user. Shared handles are
    // fine: the distance falloff keeps far-away users dark anyway.
    let hovered: HashSet<AssetId<GlassMaterial>> = q_surfaces
        .iter()
        .filter(|(_, hover, interaction)| {
            hover.is_some_and(Hovered::get)
                || interaction
                    .is_some_and(|i| !matches!(i, Interaction::None))
        })
        .map(|(node, _, _)| node.0.id())
        .collect();

    if last.0 == pos && last.1 == hovered {
        return;
    }
    *last = (pos, hovered.clone());

    let radius = 160.0 * window.scale_factor();
    for (id, material) in materials.iter_mut() {
        // Preserve `glow.w`: the per-material radius scale.
        material.glow.x = pos.x;
        material.glow.y = pos.y;
        material.glow.z = radius;
        material.params.w = if hovered.contains(&id) {
            material.base_glow
        } else {
            0.0
        };
    }
}

/// Swap button materials on hover / press.
fn update_glass_buttons(
    assets: Res<GlassAssets>,
    mut q_buttons: Query<
        (&Hovered, Has<Pressed>, &mut MaterialNode<GlassMaterial>),
        With<Button>,
    >,
) {
    for (hovered, pressed, mut node) in &mut q_buttons {
        let want = if pressed {
            &assets.button_pressed
        } else if hovered.get() {
            &assets.button_hover
        } else {
            &assets.button
        };
        if node.0 != *want {
            node.0 = want.clone();
        }
    }
}

//! Glass presets: the [`Glass`] marker, the shared material handles,
//! and the observer that attaches the right material to a node.

use bevy::prelude::*;
use bevy::ui_render::prelude::MaterialNode;

use super::material::GlassMaterial;
use crate::ui::theme::EditorTheme;

/// Declarative glass preset. Inserting one (or re-inserting a
/// different variant) swaps the node's material accordingly. Prefer
/// the [`widget`](super::widget) builders over naming variants
/// directly.
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
    /// Buttons (idle; hover/press handled by `update_glass_buttons`).
    Button,
    /// Recessed input fill (text fields, checkboxes).
    Field,
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
    pub field: Handle<GlassMaterial>,
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
            Glass::Field => self.field.clone(),
        }
    }
}

/// Build every preset material from the theme palette.
pub(super) fn build_assets(
    theme: &EditorTheme,
    materials: &mut Assets<GlassMaterial>,
) -> GlassAssets {
    let tint = |color: Color, alpha: f32| -> LinearRgba {
        color.with_alpha(alpha).to_linear()
    };
    let base = &theme.palette.base;
    let accent = theme.accent;

    // `new(tint, rim, frost_px)`: frost is the backdrop blur.
    GlassAssets {
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
        // Recessed, darker than a button so text/marks read; glows
        // under the cursor like the buttons.
        field: materials.add(
            GlassMaterial::new(tint(base[0], 0.55), 0.15, 6.0)
                .rim_opacity(0.70)
                .glow_strength(1.10)
                .glow_radius_scale(0.5),
        ),
    }
}

/// Attach / swap the material whenever a [`Glass`] preset is
/// inserted.
pub(super) fn attach_glass(
    insert: On<Insert, Glass>,
    q_glass: Query<&Glass>,
    assets: Res<GlassAssets>,
    mut materials: ResMut<Assets<GlassMaterial>>,
    mut commands: Commands,
) {
    let Ok(glass) = q_glass.get(insert.entity) else {
        return;
    };
    let preset = assets.preset(*glass);
    // Fields stack closely and share the glow falloff, so give each a
    // unique material instance to isolate its cursor glow. (Fields
    // never swap preset, so this doesn't leak on re-insert.)
    let handle = if matches!(glass, Glass::Field) {
        materials
            .get(&preset)
            .cloned()
            .map(|m| materials.add(m))
            .unwrap_or(preset)
    } else {
        preset
    };
    commands.entity(insert.entity).insert(MaterialNode(handle));
}

/// Skins the first child of an entity once it exists. Lets us glass a
/// widget whose background lives on a scene-spawned child we can't
/// reach synchronously (e.g. a feathers checkbox's box). Consumed on
/// success. Prefer
/// [`widget::glassify_child`](super::widget::glassify_child).
#[derive(Component)]
pub struct GlassifyChild(pub Glass);

pub(super) fn glassify_children(
    q: Query<(Entity, &Children, &GlassifyChild)>,
    mut commands: Commands,
) {
    for (entity, children, glassify) in &q {
        let Some(child) = children.iter().next() else {
            continue;
        };
        commands.entity(child).insert(glassify.0);
        commands.entity(entity).remove::<GlassifyChild>();
    }
}

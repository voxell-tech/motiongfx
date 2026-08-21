//! Dragging a file onto an asset field.
//!
//! What extension loads as which [`Asset`] is a registration
//! ([`AssetKinds`]), not a match on a fixed list, so a new asset type
//! is one [`AssetKindAppExt::register_asset_kind`] away. The drag
//! itself ([`AssetDragging`]) carries a path and that same kind, so a
//! drop target only has to compare one [`TypeId`] to know whether
//! what landed on it is its own.

use std::any::TypeId;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::asset::io::AssetSourceBuilder;
use bevy::asset::{Asset, AssetApp};
use bevy::picking::events::{Drag, DragEnd, DragStart, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::UiScale;

use bevy_fynix::{BevyFynix, EntityExt};
use fynix_mock::element::Element;
use fynix_mock::ui::ElementMut;

use crate::reactive::BevyHost;
use crate::theme::EditorTheme;

/// Where the ghost sits relative to the cursor, so the cursor lands
/// just inside it rather than on its corner.
const GHOST_OFFSET: Vec2 = Vec2::new(-8.0, -9.0);

/// The [`AssetSourceId`](bevy::asset::io::AssetSourceId) a dragged
/// file loads through, rooted at `/` rather than wherever
/// `AssetPlugin::file_path` put the editor's own configured root - a
/// bookmark can point anywhere on disk, not just under that root.
pub const ABSOLUTE_SOURCE: &str = "abs";

/// Registers [`ABSOLUTE_SOURCE`]. Must run before `DefaultPlugins`:
/// asset sources build when `AssetPlugin` does, not after.
pub fn register_absolute_source(app: &mut App) {
    app.register_asset_source(
        ABSOLUTE_SOURCE,
        AssetSourceBuilder::platform_default("/", None),
    );
}

/// Which file extension loads as which [`Asset`], by that asset's
/// own [`TypeId`].
#[derive(Resource, Default)]
pub struct AssetKinds {
    by_extension: HashMap<String, TypeId>,
}

impl AssetKinds {
    /// The registered kind `path`'s extension loads as, if any.
    pub fn kind_of(&self, path: &Path) -> Option<TypeId> {
        let extension = path.extension()?.to_str()?.to_lowercase();
        self.by_extension.get(&extension).copied()
    }
}

/// Registering what a file extension loads as.
pub trait AssetKindAppExt {
    /// Marks every extension in `extensions` as loading a `T`.
    fn register_asset_kind<T: Asset>(
        &mut self,
        extensions: &[&str],
    ) -> &mut Self;
}

impl AssetKindAppExt for App {
    fn register_asset_kind<T: Asset>(
        &mut self,
        extensions: &[&str],
    ) -> &mut Self {
        let mut kinds = self
            .world_mut()
            .get_resource_or_insert_with(AssetKinds::default);
        for extension in extensions {
            kinds
                .by_extension
                .insert(extension.to_lowercase(), TypeId::of::<T>());
        }
        self
    }
}

/// The file being dragged, its kind, and what is following the cursor
/// meanwhile. Empty whenever nothing is being dragged.
#[derive(Resource, Default)]
pub struct AssetDragging {
    pub path: Option<PathBuf>,
    pub kind: Option<TypeId>,
    ghost: Option<Entity>,
}

/// Makes `elem` a file that can be picked up and dragged onto an
/// asset field, named `label` while it follows the cursor.
pub fn draggable<'r, 'u, 'a, E: Element<BevyHost>>(
    elem: &'r mut ElementMut<'u, 'a, BevyHost, E>,
    path: PathBuf,
    kind: TypeId,
    label: String,
) -> &'r mut ElementMut<'u, 'a, BevyHost, E> {
    elem.observe(
        move |start: On<Pointer<DragStart>>,
              kernel: Res<BevyFynix<EditorTheme>>,
              scale: Res<UiScale>,
              mut dragging: ResMut<AssetDragging>,
              mut commands: Commands| {
            if start.button != PointerButton::Primary {
                return;
            }

            let at = start.pointer_location.position / scale.0;

            dragging.path = Some(path.clone());
            dragging.kind = Some(kind);
            dragging.ghost = Some(
                commands
                    .spawn(ghost(at, label.clone(), kernel.theme()))
                    .id(),
            );
        },
    )
    .observe(
        move |drag: On<Pointer<Drag>>,
              scale: Res<UiScale>,
              dragging: Res<AssetDragging>,
              mut nodes: Query<&mut Node>| {
            let Some(ghost) = dragging.ghost else {
                return;
            };
            let Ok(mut node) = nodes.get_mut(ghost) else {
                return;
            };
            let at = drag.pointer_location.position / scale.0
                + GHOST_OFFSET;

            node.left = px(at.x);
            node.top = px(at.y);
        },
    )
    .observe(
        move |_: On<Pointer<DragEnd>>,
              mut dragging: ResMut<AssetDragging>,
              mut commands: Commands| {
            if let Some(ghost) = dragging.ghost.take() {
                commands.entity(ghost).despawn();
            }
            dragging.path = None;
            dragging.kind = None;
        },
    )
}

/// What follows the cursor while a file is being dragged.
///
/// [`Pickable::IGNORE`] because it sits directly under the cursor:
/// seen by the pointer it would be the only thing ever dragged over,
/// and no drop target would light up.
fn ghost(at: Vec2, name: String, theme: &EditorTheme) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x + GHOST_OFFSET.x),
            top: px(at.y + GHOST_OFFSET.y),
            padding: UiRect::axes(px(6), px(2)),
            border_radius: BorderRadius::all(px(3)),
            ..default()
        },
        BackgroundColor(theme.accent.with_alpha(0.85)),
        GlobalZIndex(200),
        Pickable::IGNORE,
        children![(
            Text::new(name),
            TextFont {
                font_size: FontSize::Px(12.0),
                ..default()
            },
            TextColor(theme.palette.base[0]),
            TextLayout::linebreak(LineBreak::NoWrap),
            Pickable::IGNORE,
        )],
    )
}

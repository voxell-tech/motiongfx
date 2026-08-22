use std::any::TypeId;
use std::collections::HashMap;
use std::path::Path;

use bevy::asset::io::AssetSourceBuilder;
use bevy::asset::{Asset, AssetApp};
use bevy::prelude::*;

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

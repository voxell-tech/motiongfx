//! Asset infrastructure the editor's UI builds on, but that has
//! nothing to do with UI itself: what file extension loads as which
//! [`Asset`](bevy::asset::Asset) ([`AssetKinds`]), the source a
//! bookmark's absolute path resolves through ([`ABSOLUTE_SOURCE`]),
//! and the `.mat` file format ([`StdMaterialAssetLoader`]).
//!
//! [`MoxieAssetPlugin`] wires all of it onto an [`App`] at once; a
//! consumer that only wants one piece can still reach it directly.

mod registry;
mod std_material;

use bevy::asset::io::AssetSourceBuilder;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::prelude::*;

pub use registry::{AssetKindAppExt, AssetKinds};
pub use std_material::{StdMaterialAssetLoader, serialize_to_ron};

pub struct MoxieAssetPlugin;

impl Plugin for MoxieAssetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetKinds>();

        let registry =
            app.world().resource::<AppTypeRegistry>().clone();
        app.register_asset_loader(StdMaterialAssetLoader::new(
            &registry,
        ));
    }
}

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

//! Asset infrastructure the editor's UI builds on, but that has
//! nothing to do with UI itself: what file extension loads as which
//! [`Asset`](bevy::asset::Asset) ([`AssetKinds`]), the source a
//! bookmark's absolute path resolves through ([`ABSOLUTE_SOURCE`]),
//! and the `.mat` file format ([`StdMaterialAssetLoader`]).

mod registry;
mod std_material;

pub use registry::{
    ABSOLUTE_SOURCE, AssetKindAppExt, AssetKinds,
    register_absolute_source,
};
pub use std_material::{StdMaterialAssetLoader, serialize_to_ron};

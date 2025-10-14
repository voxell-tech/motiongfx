use bevy_asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy_log::prelude::*;
use bevy_utils::BoxedFuture;
use bevy_vello_renderer::vello_svg::usvg;

use crate::bezpath::BezPathGroup;

#[derive(Default)]
pub struct VelloSvgLoader;

impl AssetLoader for VelloSvgLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy_asset::Error>> {
        Box::pin(async move {
            debug!("parsing {}...", load_context.path().display());

            // Deserialize the SVG source into `BezPathGroup`
            let tree: usvg::Tree = usvg::Tree::from_data(&bytes, &usvg::Options::default())?;
            let bezpath_group: BezPathGroup = BezPathGroup::from_tree(&tree);

            info!(
                path = format!("{}", load_context.path().display()),
                // size = format!("{:?}", (bezpath_group.size.x, bezpath_group.size.y)),
                "finished parsing svg asset"
            );

            load_context.set_default_asset(LoadedAsset::new(bezpath_group));

            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["svg", "svg.gz"]
    }
}

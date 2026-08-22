//! [`Inspect`] for a [`Handle<T>`], for any asset `T`.
//!
//! One row, showing whatever asset is currently assigned, and a drop
//! target for a file dragged from the assets panel whose registered
//! [`moxie_asset::AssetKinds`] kind matches `T`.

use std::any::TypeId;

use bevy::asset::{Asset, AssetPath};
use bevy::picking::events::{DragDrop, Pointer};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;

use bevy_fynix::EntityExt;
use fynix_mock::{elem, val};
use moxie_asset::ABSOLUTE_SOURCE;

use crate::asset::AssetDragging;
use crate::elements::{
    ButtonElemCursor, GhostButton, Icon, Label, LabelCursor,
};
use crate::icons;
use crate::reactive::BevyUi;

use super::{Inspect, Source, SourceExt, when_changed};

impl<T: Asset + TypePath> Inspect for Handle<T> {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        let read = source.boxed();
        let written = source.boxed();
        let muted = ui.theme.text_muted;
        let kind = TypeId::of::<T>();
        let label = label_of::<T>(ui.world, &*read);

        ui.elem(elem!(
            !GhostButton,
            width = percent(100),
            justify = JustifyContent::SpaceBetween,
            icon = val!(Icon, image = icons::ASSET, color = muted),
            label = val!(
                Label,
                text = label,
                color = Some(muted),
                wrap = false
            )
        ))
        .bind(
            |button| button.label().text(),
            when_changed(source),
            move |world, _| label_of::<T>(world, &*read),
        )
        .observe(
            move |drop: On<Pointer<DragDrop>>,
                  dragging: Res<AssetDragging>,
                  mut commands: Commands| {
                if drop.button != PointerButton::Primary {
                    return;
                }
                let (Some(path), Some(dragged)) =
                    (dragging.path.clone(), dragging.kind)
                else {
                    return;
                };
                if dragged != kind {
                    return;
                }

                let source = written.boxed();
                commands.queue(move |world: &mut World| {
                    // Rooted at `/`, not wherever `AssetPlugin`
                    // configured its own root - a dragged path is
                    // absolute and may live anywhere on disk.
                    let asset_path =
                        AssetPath::from_path_buf(path.clone())
                            .with_source(ABSOLUTE_SOURCE);
                    // A dragged file's path is outside the
                    // configured asset root by construction, so it
                    // needs `Deny`'s per-load override; see
                    // `unapproved_path_mode` in `main.rs`.
                    let handle = world
                        .resource::<AssetServer>()
                        .load_builder()
                        .override_unapproved()
                        .load::<T>(asset_path);
                    source.write(world, handle);
                });
            },
        );
    }
}

/// What `source` currently holds, as a path to show - the asset's own
/// name if the server knows one, or a placeholder for a handle with
/// none or nothing assigned at all.
fn label_of<T: Asset>(world: &World, source: &dyn Source) -> String {
    let Some(handle) = source.read::<Handle<T>>(world) else {
        return "(none)".to_string();
    };

    world
        .get_resource::<AssetServer>()
        .and_then(|assets| assets.get_path(&handle))
        .map(|path| path.path().display().to_string())
        .unwrap_or_else(|| "(unnamed)".to_string())
}

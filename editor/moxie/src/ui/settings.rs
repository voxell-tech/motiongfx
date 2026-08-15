//! The settings panel: a reflect inspector over [`EditorSettings`],
//! and the button that writes it back to disk.

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::settings::SaveSettingsSync;
use bevy_fynix::ElementMutExt;
use fynix_mock::composer::Composer;
use fynix_mock::ui::ElementHandle;
use fynix_mock::{elem, val};
use moxie_ui::elements::{
    Button, Frame, Label, Panel, ResourceInspector,
};
use moxie_ui::reactive::{BevyHost, BevyUi};

use super::PANEL_PADDING;
use crate::EditorSettings;

/// The settings panel, as kernel nodes.
pub(super) struct SettingsPanel;

impl Composer<BevyHost> for SettingsPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            row_gap = px(8),
            padding = UiRect::all(px(PANEL_PADDING)),
            scrolls = true
        ))
        .with(|ui| {
            // Editable rows built by the reflect inspector.
            ui.compose(ResourceInspector::of::<EditorSettings>());
            ui.compose(SaveRow);
        })
        .handle()
    }
}

/// The one action the panel has of its own.
struct SaveRow;

impl Composer<BevyHost> for SaveRow {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        ui.elem(elem!(Frame, direction = FlexDirection::Row))
            .with(|ui| {
                ui.elem(elem!(
                    !Button,
                    label = val!(Label, text = "Save"),
                    width = px(64),
                    height = px(24)
                ))
                .observe(
                    |mut click: On<Pointer<Click>>,
                     mut commands: Commands| {
                        click.propagate(false);
                        commands.queue(SaveSettingsSync::Always);
                    },
                );
            })
            .handle()
    }
}

//! The settings panel: a reflect inspector over [`EditorSettings`],
//! and the button that writes it back to disk.

use bevy::picking::events::{Click, Pointer};
use bevy::prelude::*;
use bevy::settings::SaveSettingsSync;
use bevy_fynix::WorldEntityMut;
use fynix::composer::Composer;
use fynix::elem;
use fynix::ui::ElementHandle;
use moxie_ui::elements::{
    Button, Frame, Label, Panel, ResourceInspector,
};
use moxie_ui::reactive::{BevyUi, FynixHost};

use crate::EditorSettings;

/// The settings panel, as kernel nodes.
pub(super) struct SettingsPanel;

impl Composer<FynixHost> for SettingsPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Panel> {
        let pad = ui.theme.space.xl;
        ui.elem(elem!(
            Panel,
            direction = FlexDirection::Column,
            row_gap = px(8),
            padding = UiRect::all(px(pad)),
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

impl Composer<FynixHost> for SaveRow {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
        ui.elem(elem!(Frame, direction = FlexDirection::Row))
            .with(|ui| {
                ui.elem(elem!(
                    Button,
                    label = elem!(Label, text = "Save"),
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

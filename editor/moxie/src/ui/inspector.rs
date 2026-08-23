//! Inspects whatever is selected in the hierarchy: every reflectable
//! component of one entity, each under a collapsible header.

use bevy::prelude::*;
use fynix_mock::composer::Composer;
use fynix_mock::elem;
use fynix_mock::ui::ElementHandle;
use moxie_ui::elements::{EntityInspector, Label, ScrollArea};
use moxie_ui::reactive::{BevyHost, BevyUi, resource_changed};

use super::PANEL_PADDING;
use crate::SelectedEntity;

/// The inspector panel, as kernel nodes.
pub(super) struct InspectorPanel;

impl Composer<BevyHost> for InspectorPanel {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, ScrollArea> {
        ui.elem(elem!(
            ScrollArea,
            width = percent(100),
            flex_grow = 1.0f32,
            row_gap = px(8),
            padding = UiRect::all(px(PANEL_PADDING)),
            scroll_x = false
        ))
        .watch(resource_changed::<SelectedEntity>(), build)
        .handle()
    }
}

fn build(ui: &mut BevyUi) {
    let Some(entity) = ui.world.resource::<SelectedEntity>().0 else {
        let muted = ui.theme.text_muted;
        ui.elem(elem!(
            Label,
            text = "Nothing selected",
            color = Some(muted)
        ));
        return;
    };
    ui.compose(EntityInspector { entity });
}

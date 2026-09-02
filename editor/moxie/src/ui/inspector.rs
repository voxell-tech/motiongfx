//! Inspects whatever is selected in the hierarchy: every reflectable
//! component of one entity, each under a collapsible header.

use bevy::prelude::*;
use fynix::composer::Composer;
use fynix::elem;
use fynix::ui::ElementHandle;
use moxie_ui::elements::{EntityInspector, Label, ScrollArea};
use moxie_ui::reactive::{BevyUi, FynixHost, resource_changed};

use super::PANEL_PADDING;
use crate::SelectedEntity;

/// The inspector panel, as kernel nodes.
pub(super) struct InspectorPanel;

impl Composer<FynixHost> for InspectorPanel {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, ScrollArea> {
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
            color = muted
        ));
        return;
    };
    ui.compose(EntityInspector { entity });
}

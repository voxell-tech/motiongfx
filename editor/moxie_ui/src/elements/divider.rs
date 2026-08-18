use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::ControlOrientation;
use bevy::window::SystemCursorIcon;
use bevy_fynix::BevyUi;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

const DIVIDER_WIDTH: f32 = 6.0;

/// The draggable line between two panes.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Divider {
    #[default(px(DIVIDER_WIDTH))]
    pub thickness: Val,
    #[default(::Horizontal)]
    pub orientation: ControlOrientation,
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.08))]
    pub color: Color,
}

impl Divider {
    /// The two that the orientation decides between: which way the
    /// line runs, and what the cursor says it will do.
    fn shape(&self) -> (Node, SystemCursorIcon) {
        let (width, height, cursor) = match self.orientation {
            ControlOrientation::Horizontal => (
                percent(100),
                self.thickness,
                SystemCursorIcon::NsResize,
            ),
            ControlOrientation::Vertical => (
                self.thickness,
                percent(100),
                SystemCursorIcon::EwResize,
            ),
        };

        (
            Node {
                width,
                height,
                flex_shrink: 0.0,
                ..default()
            },
            cursor,
        )
    }
}

impl ElementVisual<BevyHost> for Divider {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        let (layout, cursor) = self.shape();

        world.entity_mut(node).insert((
            layout,
            BackgroundColor(self.color),
            EntityCursor::System(cursor),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: DividerField,
    ) {
        match field {
            // Either one changes both: the thickness lands on
            // whichever axis the orientation put it.
            DividerField::Thickness | DividerField::Orientation => {
                let (layout, cursor) = self.shape();

                world
                    .entity_mut(node)
                    .insert((layout, EntityCursor::System(cursor)));
            }
            DividerField::Color => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.color));
            }
        }
    }
}

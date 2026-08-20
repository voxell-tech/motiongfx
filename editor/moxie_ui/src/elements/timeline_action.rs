use crate::reactive::BevyHost;
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::EntityExt as _;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::{Build, Patch};

/// One action's clip on the timeline: a colored, absolutely
/// positioned, bordered hit area.
#[derive(Element)]
pub struct TimelineAction {
    pub top: f32,
    pub left: f32,
    pub width: f32,
    pub height: f32,
    #[default(Color::NONE)]
    pub fill: Color,
    #[default(Color::NONE)]
    pub border: Color,
    /// Thickens the border - the caller still chooses `border`'s
    /// color (the theme's accent, typically).
    pub selected: bool,
}

impl TimelineAction {
    fn node(&self) -> Node {
        Node {
            position_type: PositionType::Absolute,
            top: px(self.top),
            left: px(self.left),
            width: px(self.width),
            height: px(self.height),
            border: UiRect::all(px(if self.selected {
                2
            } else {
                1
            })),
            ..default()
        }
    }
}

impl ElementVisual<BevyHost> for TimelineAction {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((
            self.node(),
            BackgroundColor(self.fill),
            BorderColor::all(self.border),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: TimelineActionField,
    ) {
        match field {
            TimelineActionField::Top
            | TimelineActionField::Left
            | TimelineActionField::Width
            | TimelineActionField::Height
            | TimelineActionField::Selected => {
                patch.insert(self.node());
            }
            TimelineActionField::Fill => {
                patch.insert(BackgroundColor(self.fill));
            }
            TimelineActionField::Border => {
                patch.insert(BorderColor::all(self.border));
            }
        }
    }
}

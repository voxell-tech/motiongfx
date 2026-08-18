use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::BevyUi;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// What a clip's fill brightens to under the cursor, and further
/// while held - a saturated blue, deliberately not the editor's usual
/// gray hover overlay: a clip is a colored timeline object in its own
/// right, not chrome, so it lights up in its own family of color.
///
/// Public so a call site can pair it with [`MotionExt::lit`](
/// crate::motion::MotionExt::lit): the element itself only owns its
/// look, not its own interaction wiring.
pub const HOVER_TINT: Color = Color::srgb(0.35, 0.70, 1.0);
pub const PRESS_TINT: Color = Color::srgb(0.55, 0.82, 1.0);

/// One action's clip on the timeline: a colored, absolutely
/// positioned, bordered hit area.
#[derive(Element, OverrideDefault, Lenz)]
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
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.fill),
            BorderColor::all(self.border),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TimelineActionField,
    ) {
        match field {
            TimelineActionField::Top
            | TimelineActionField::Left
            | TimelineActionField::Width
            | TimelineActionField::Height
            | TimelineActionField::Selected => {
                world.entity_mut(node).insert(self.node());
            }
            TimelineActionField::Fill => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.fill));
            }
            TimelineActionField::Border => {
                world
                    .entity_mut(node)
                    .insert(BorderColor::all(self.border));
            }
        }
    }
}

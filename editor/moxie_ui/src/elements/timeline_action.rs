use bevy::feathers::cursor::EntityCursor;
use bevy::picking::events::{
    Cancel, Out, Over, Pointer, Press, Release,
};
use bevy::prelude::*;
use bevy::ui_widgets::Button as ButtonBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// What a clip's fill brightens to under the cursor, and further
/// while held - a saturated blue, deliberately not the editor's usual
/// gray hover overlay: a clip is a colored timeline object in its own
/// right, not chrome, so it lights up in its own family of color.
const HOVER_TINT: Color = Color::srgb(0.35, 0.70, 1.0);
const PRESS_TINT: Color = Color::srgb(0.55, 0.82, 1.0);

/// One action's clip on the timeline: a colored, absolutely
/// positioned, bordered hit area. Owns its interaction directly - a
/// pointer cursor and a hover/press tint via observers on its own
/// entity - rather than reaching for the editor's generic button
/// motion, since a clip's hover color is a deliberately different
/// look, not the shared gray overlay every chrome button uses.
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

/// The clip's resting fill, so the hover/press observers know what to
/// fade back to on the way out.
#[derive(Component)]
struct BaseFill(Color);

impl ElementVisual<BevyHost> for TimelineAction {
    fn build_fields(&self, world: &mut World, node: Entity) {
        world.entity_mut(node).insert((
            self.node(),
            BackgroundColor(self.fill),
            BorderColor::all(self.border),
            BaseFill(self.fill),
            ButtonBehavior,
            EntityCursor::System(SystemCursorIcon::Pointer),
        ));

        world
            .entity_mut(node)
            .observe(on_over)
            .observe(on_out)
            .observe(on_press)
            .observe(on_release)
            .observe(on_cancel);
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
                world.entity_mut(node).insert((
                    BackgroundColor(self.fill),
                    BaseFill(self.fill),
                ));
            }
            TimelineActionField::Border => {
                world
                    .entity_mut(node)
                    .insert(BorderColor::all(self.border));
            }
        }
    }
}

fn on_over(
    over: On<Pointer<Over>>,
    mut q: Query<&mut BackgroundColor>,
) {
    if let Ok(mut background) = q.get_mut(over.entity) {
        background.0 = HOVER_TINT;
    }
}

fn on_press(
    press: On<Pointer<Press>>,
    mut q: Query<&mut BackgroundColor>,
) {
    if let Ok(mut background) = q.get_mut(press.entity) {
        background.0 = PRESS_TINT;
    }
}

fn on_release(
    release: On<Pointer<Release>>,
    mut q: Query<&mut BackgroundColor>,
) {
    if let Ok(mut background) = q.get_mut(release.entity) {
        background.0 = HOVER_TINT;
    }
}

fn on_out(
    out: On<Pointer<Out>>,
    q_base: Query<&BaseFill>,
    mut q_background: Query<&mut BackgroundColor>,
) {
    reset(out.entity, &q_base, &mut q_background);
}

fn on_cancel(
    cancel: On<Pointer<Cancel>>,
    q_base: Query<&BaseFill>,
    mut q_background: Query<&mut BackgroundColor>,
) {
    reset(cancel.entity, &q_base, &mut q_background);
}

/// Back to the resting fill: what `Out`/`Cancel` share.
fn reset(
    entity: Entity,
    q_base: &Query<&BaseFill>,
    q_background: &mut Query<&mut BackgroundColor>,
) {
    if let (Ok(base), Ok(mut background)) =
        (q_base.get(entity), q_background.get_mut(entity))
    {
        background.0 = base.0;
    }
}

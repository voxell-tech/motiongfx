//! What an inspector row is edited with.

use crate::reactive::FynixBuild;
use bevy::feathers::controls::{
    FeathersNumberInput, FeathersTextInput,
    FeathersTextInputContainer, NumberFormat, NumberInputValue,
    UpdateNumberInput,
};
use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::scene::EntityWorldMutSceneExt;
use bevy::text::EditableText;
use bevy::ui::Checked;
use bevy::ui_widgets::Checkbox as CheckboxBehavior;
use bevy::window::SystemCursorIcon;
use bevy_fynix::WorldEntityMut;
use fynix::element::element;

use super::patch::*;

/// A box that is ticked or not.
#[element(build = Self::build)]
pub struct CheckBox {
    #[elem(patch = PatchChecked)]
    pub checked: bool,
    #[elem(patch = PatchBackground)]
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.08))]
    pub fill: Color,
    #[elem(patch = PatchMark)]
    #[default(Color::srgb(0.47, 0.86, 0.91))]
    pub mark: Color,
}

/// The inner square a ticked box shows, on a child of its own: a node
/// draws one background, and the box's own is the field it sits in.
#[derive(Component)]
struct CheckMark;

/// Toggle the `Checked` marker and show or hide the inner square.
pub(super) fn tick(checked: bool, entity: &mut impl WorldEntityMut) {
    if checked {
        entity.insert(Checked);
    } else {
        entity.remove::<Checked>();
    }

    let node = entity.id();
    let world = entity.world_mut();
    let Some(mark) = mark_node(world, node) else {
        return;
    };
    if let Some(mut layout) = world.get_mut::<Node>(mark) {
        layout.display = if checked {
            Display::Flex
        } else {
            Display::None
        };
    }
}

/// Paint the inner square.
pub(super) fn paint(mark: Color, entity: &mut impl WorldEntityMut) {
    let node = entity.id();
    let world = entity.world_mut();
    let Some(spot) = mark_node(world, node) else {
        return;
    };
    world.entity_mut(spot).insert(BackgroundColor(mark));
}

/// The mark the build hook spawned, found by its marker rather than
/// by position: a box may be given children of its own.
fn mark_node(world: &World, node: Entity) -> Option<Entity> {
    world
        .get::<Children>(node)?
        .iter()
        .find(|&child| world.get::<CheckMark>(child).is_some())
}

impl CheckBox {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        build
            .insert((
                CheckboxBehavior,
                EntityCursor::System(SystemCursorIcon::Pointer),
                Node {
                    width: px(16),
                    height: px(16),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(px(4)),
                    ..default()
                },
                BackgroundColor(self.fill),
            ))
            .with_child((
                CheckMark,
                Node {
                    width: px(8),
                    height: px(8),
                    border_radius: BorderRadius::all(px(2)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(self.mark),
            ));

        tick(self.checked, build);
    }
}

field_patch!(PatchChecked, bool, |patch, v| tick(*v, patch));

field_patch!(PatchMark, Color, |patch, v| paint(*v, patch));

/// A number, typed or dragged.
#[element(build = Self::build)]
pub struct NumberField {
    #[elem(patch = PatchNumberFormat)]
    pub format: NumberFormat,
    /// What it shows. Pushed as an event rather than written as a
    /// component, so an input being typed into ignores it and a live
    /// edit wins.
    #[elem(patch = PatchNumberValue)]
    #[default(NumberInputValue::F32(0.0))]
    pub value: NumberInputValue,
    #[elem(patch = PatchWidth)]
    #[default(px(80))]
    pub width: Val,
}

/// Feathers builds the input as a scene of its own: a container, the
/// two steppers, and the text in between. Widening the node it wrote,
/// not replacing it - the container carries the row's height, padding
/// and rim, and an input without them has nothing to type into.
pub(super) fn number_scene(
    format: NumberFormat,
    width: Val,
    entity: &mut impl WorldEntityMut,
) {
    let scene = bsn! {
        @FeathersNumberInput { @number_format: {format} }
    };
    if let Err(err) = entity.entity_mut().apply_scene(scene) {
        error!("failed to build a number field: {err}");
    }
    if let Some(mut layout) = entity.entity_mut().get_mut::<Node>() {
        layout.width = width;
        layout.flex_grow = 0.0;
    }
}

impl NumberField {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        number_scene(self.format, self.width, build);
    }
}

field_patch!(PatchNumberFormat, NumberFormat, |patch, v| {
    let width = patch
        .entity_mut()
        .get::<Node>()
        .map(|node| node.width)
        .unwrap_or(px(80));
    number_scene(*v, width, patch);
});

field_patch!(PatchNumberValue, NumberInputValue, |patch, v| {
    let node = patch.id();
    patch.world.trigger(UpdateNumberInput {
        entity: node,
        value: *v,
    });
});

/// A single-line string, edited in place.
#[element(build = Self::build)]
pub struct TextField {
    #[elem(patch = PatchTextValue)]
    pub value: String,
    #[elem(patch = PatchWidth)]
    #[default(px(110))]
    pub width: Val,
}

impl TextField {
    /// The child entity actually holding [`EditableText`], found by
    /// marker rather than position - the container may end up with
    /// other children later (an icon, say), and nothing here should
    /// depend on which one comes first. What a caller reaching past
    /// this element's own fields (to wire an observer directly, say)
    /// needs too.
    pub fn text_input(world: &World, node: Entity) -> Option<Entity> {
        world
            .get::<Children>(node)?
            .iter()
            .find(|&child| world.get::<EditableText>(child).is_some())
    }
}

/// Feathers wants its own child entity for the editable text - see
/// the type's own docs - so this node is the container, and the
/// widget beneath it what actually holds [`EditableText`].
fn text_scene(
    width: Val,
    value: &str,
    entity: &mut impl WorldEntityMut,
) {
    let scene = bsn! {
        @FeathersTextInputContainer
        Children [
            ( @FeathersTextInput )
        ]
    };
    if let Err(err) = entity.entity_mut().apply_scene(scene) {
        error!("failed to build a text field: {err}");
    }

    if let Some(mut layout) = entity.entity_mut().get_mut::<Node>() {
        layout.width = width;
        layout.flex_grow = 0.0;

        // `FeathersTextInputContainer` reserves its left inset as a
        // colorless 3px *border* rather than padding (room for a
        // leading icon we never add), which leaves the background
        // unpainted there and the left corners looking square next to
        // the fully rounded right ones. Folding it into padding
        // instead paints the background - and so the radius - all the
        // way around.
        layout.border = UiRect::ZERO;
        layout.padding = UiRect::horizontal(px(3.0));
    }

    set_text(value, entity);
}

/// Write `value` into the child [`EditableText`].
pub(super) fn set_text(
    value: &str,
    entity: &mut impl WorldEntityMut,
) {
    let node = entity.id();
    let world = entity.world_mut();
    let Some(input) = TextField::text_input(world, node) else {
        return;
    };
    if let Some(mut text) = world.get_mut::<EditableText>(input) {
        text.editor_mut().set_text(value);
    }
}

impl TextField {
    fn build(&self, build: &mut FynixBuild<'_, Self>) {
        text_scene(self.width, &self.value, build);
    }
}

field_patch!(PatchTextValue, String, |patch, v| set_text(v, patch));

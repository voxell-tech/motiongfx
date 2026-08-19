//! What an inspector row is edited with.

use crate::reactive::BevyHost;
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
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::ui::Draw;

/// A box that is ticked or not.
///
/// `checked` is a field rather than something written from outside,
/// so the value it shows follows whatever the world says: bevy tracks
/// it as the presence of a marker, and this turns that into data.
#[derive(Element)]
pub struct CheckBox {
    pub checked: bool,
    #[default(Color::srgba(1.0, 1.0, 1.0, 0.08))]
    pub fill: Color,
    #[default(Color::srgb(0.47, 0.86, 0.91))]
    pub mark: Color,
}

/// The inner square a ticked box shows, on a child of its own: a node
/// draws one background, and the box's own is the field it sits in.
#[derive(Component)]
struct CheckMark;

impl CheckBox {
    fn tick(&self, world: &mut World, node: Entity) {
        let mut entity = world.entity_mut(node);

        if self.checked {
            entity.insert(Checked);
        } else {
            entity.remove::<Checked>();
        }

        let Some(mark) = self.mark_node(world, node) else {
            return;
        };
        if let Some(mut layout) = world.get_mut::<Node>(mark) {
            layout.display = if self.checked {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    fn paint(&self, world: &mut World, node: Entity) {
        let Some(mark) = self.mark_node(world, node) else {
            return;
        };
        world.entity_mut(mark).insert(BackgroundColor(self.mark));
    }

    /// The mark [`build_fields`](ElementVisual::build_fields) spawned,
    /// found by its marker rather than by position: a box may be given
    /// children of its own.
    fn mark_node(
        &self,
        world: &World,
        node: Entity,
    ) -> Option<Entity> {
        world
            .get::<Children>(node)?
            .iter()
            .find(|&child| world.get::<CheckMark>(child).is_some())
    }
}

impl ElementVisual<BevyHost> for CheckBox {
    fn build_fields(&self, element: &mut Draw<'_, BevyHost, Self>) {
        let node = element.id();
        let world = &mut *element.world;

        world.entity_mut(node).insert((
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
        ));

        let mark = world
            .spawn((
                CheckMark,
                Node {
                    width: px(8),
                    height: px(8),
                    border_radius: BorderRadius::all(px(2)),
                    display: Display::None,
                    ..default()
                },
                BackgroundColor(self.mark),
            ))
            .id();
        world.entity_mut(node).add_child(mark);

        self.tick(world, node);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: CheckBoxField,
    ) {
        match field {
            CheckBoxField::Checked => self.tick(world, node),
            CheckBoxField::Fill => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.fill));
            }
            CheckBoxField::Mark => self.paint(world, node),
        }
    }
}

/// A number, typed or dragged.
#[derive(Element)]
pub struct NumberField {
    pub format: NumberFormat,
    /// What it shows. Pushed as an event rather than written as a
    /// component, so an input being typed into ignores it and a live
    /// edit wins.
    #[default(NumberInputValue::F32(0.0))]
    pub value: NumberInputValue,
    #[default(px(110))]
    pub width: Val,
}

impl NumberField {
    /// Feathers builds the input as a scene of its own: a container,
    /// the two steppers, and the text in between.
    fn scene(&self, world: &mut World, node: Entity) {
        let format = self.format;
        let scene = bsn! {
            @FeathersNumberInput { @number_format: {format} }
        };

        if let Err(err) = world.entity_mut(node).apply_scene(scene) {
            error!("failed to build a number field: {err}");
        }

        // Widening the node feathers wrote, not replacing it: the
        // container carries the row's height, padding and rim, and an
        // input without them has nothing to type into.
        if let Some(mut layout) = world.get_mut::<Node>(node) {
            layout.width = self.width;
            layout.flex_grow = 0.0;
        }
    }
}

impl ElementVisual<BevyHost> for NumberField {
    fn build_fields(&self, element: &mut Draw<'_, BevyHost, Self>) {
        let node = element.id();
        let world = &mut *element.world;

        self.scene(world, node);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: NumberFieldField,
    ) {
        match field {
            NumberFieldField::Format => self.scene(world, node),
            NumberFieldField::Value => {
                world.trigger(UpdateNumberInput {
                    entity: node,
                    value: self.value,
                });
            }
            NumberFieldField::Width => {
                if let Some(mut layout) = world.get_mut::<Node>(node)
                {
                    layout.width = self.width;
                }
            }
        }
    }
}

/// A single-line string, edited in place.
#[derive(Element)]
pub struct TextField {
    pub value: String,
    #[default(px(110))]
    pub width: Val,
}

impl TextField {
    /// Feathers wants its own child entity for the editable text -
    /// see the type's own docs - so this node is the container, and
    /// the widget beneath it what actually holds [`EditableText`].
    fn scene(&self, world: &mut World, node: Entity) {
        let scene = bsn! {
            @FeathersTextInputContainer
            Children [
                ( @FeathersTextInput )
            ]
        };

        if let Err(err) = world.entity_mut(node).apply_scene(scene) {
            error!("failed to build a text field: {err}");
        }

        if let Some(mut layout) = world.get_mut::<Node>(node) {
            layout.width = self.width;
            layout.flex_grow = 0.0;
        }

        self.show(world, node);
    }

    /// Writes `self.value` into the child [`EditableText`].
    fn show(&self, world: &mut World, node: Entity) {
        let Some(text_input) = Self::text_input(world, node) else {
            return;
        };
        if let Some(mut text) =
            world.get_mut::<EditableText>(text_input)
        {
            text.editor_mut().set_text(&self.value);
        }
    }

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

impl ElementVisual<BevyHost> for TextField {
    fn build_fields(&self, element: &mut Draw<'_, BevyHost, Self>) {
        let node = element.id();
        let world = &mut *element.world;

        self.scene(world, node);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: TextFieldField,
    ) {
        match field {
            TextFieldField::Value => self.show(world, node),
            TextFieldField::Width => {
                if let Some(mut layout) = world.get_mut::<Node>(node)
                {
                    layout.width = self.width;
                }
            }
        }
    }
}

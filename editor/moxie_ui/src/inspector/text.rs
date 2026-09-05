use bevy::prelude::*;
use bevy::text::{EditableText, TextEditChange};

use fynix::WorldNodeRef;
use fynix::elem;

use crate::elements::{TextField, TextFieldCursor};
use crate::reactive::BevyUi;

use super::{Inspect, Source, SourceExt, when_changed};

/// A single-line text input.
fn text_field<T: FromReflect>(
    source: &dyn Source,
    ui: &mut BevyUi,
    to_value: fn(String) -> T,
    to_shown: fn(&T) -> String,
) {
    let edited = source.boxed();
    let read = source.boxed();
    let shown = read
        .read::<T>(ui.world)
        .as_ref()
        .map(to_shown)
        .unwrap_or_default();

    let mut field =
        ui.elem(elem!(TextField, value = shown, width = px(110)));
    let node = field.id();

    field.bind(
        |input| input.value(),
        when_changed(source),
        move |WorldNodeRef { world, .. }| {
            read.read::<T>(world)
                .as_ref()
                .map(to_shown)
                .unwrap_or_default()
        },
    );

    let Some(text_input) = TextField::text_input(ui.world, node)
    else {
        return;
    };

    // `TextEditChange` also fires on a bare cursor move; `write` is
    // what keeps that from writing back a value that has not
    // actually changed.
    ui.world.entity_mut(text_input).observe(
        move |change: On<TextEditChange>,
              texts: Query<&EditableText>,
              mut commands: Commands| {
            let Ok(text) = texts.get(change.event_target()) else {
                return;
            };
            let (source, value) =
                (edited.boxed(), text.value().to_string());

            commands.queue(move |world: &mut World| {
                source.write(world, to_value(value));
            });
        },
    );
}

impl Inspect for String {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        text_field(source, ui, |value| value, String::clone);
    }
}

impl Inspect for Name {
    fn build(source: &dyn Source, ui: &mut BevyUi) {
        text_field(source, ui, Name::new, |name| {
            name.as_str().to_string()
        });
    }
}

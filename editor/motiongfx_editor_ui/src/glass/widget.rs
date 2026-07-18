//! Glass *widget* builders (spawn a whole thing). Styling for a node
//! you already built is the [`Glass`] marker component itself
//! (`Glass::Panel`, `Glass::tab(active)`, ...), not a builder here.

use bevy::feathers::controls::{FeathersNumberInput, NumberFormat};
use bevy::feathers::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::TextCursorStyle;
use bevy::ui::Checked;
use bevy::ui_widgets::{Button, Checkbox};
use bevy::window::SystemCursorIcon;

use super::preset::Glass;
use crate::theme::EditorTheme;

/// Glass button on the headless [`Button`]; append your own
/// `Node`/`Children`/marker. Emits [`bevy::ui_widgets::Activate`].
pub fn glass_button() -> impl Scene {
    bsn! {
        template_value(Glass::Button)
        Button
        Hovered
        EntityCursor::System(SystemCursorIcon::Pointer)
    }
}

/// Marker on a glass checkbox's inner check mark; shown/hidden and
/// colored by [`update_glass_checkmarks`].
#[derive(Component, Default, Clone)]
pub(super) struct GlassCheckMark;

/// A glass checkbox on the headless [`Checkbox`] behavior; emits
/// [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) on toggle. The
/// caller owns [`Checked`] (insert it for an initially-checked box).
pub fn glass_checkbox() -> impl Scene {
    bsn! {
        Checkbox
        Hovered
        template_value(Glass::Field)
        EntityCursor::System(SystemCursorIcon::Pointer)
        Node {
            width: Val::Px(16.0),
            height: Val::Px(16.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(4.0)),
        }
        Children [(
            GlassCheckMark
            // Shown by `update_glass_checkmarks` when checked.
            Node {
                width: Val::Px(8.0),
                height: Val::Px(8.0),
                border_radius: BorderRadius::all(Val::Px(2.0)),
                display: Display::None,
            }
            BackgroundColor(Color::NONE)
        )]
    }
}

/// A glass-styled number input: a [`FeathersNumberInput`] with
/// `Glass::Field` on its own root (the feathers text-input container).
pub fn glass_number_field(format: NumberFormat) -> impl Scene {
    bsn! {
        @FeathersNumberInput {
            @number_format: {format}
        }
        template_value(Glass::Field)
        Node {
            width: Val::Px(110.0),
            flex_grow: 0.0,
        }
    }
}

/// Theme every text field's caret from `base7` and its selection
/// highlight from the accent color.
pub(super) fn update_glass_field_cursors(
    theme: Res<EditorTheme>,
    mut q: Query<&mut TextCursorStyle>,
) {
    let caret = theme.palette.base[7];
    for mut style in &mut q {
        if style.color != caret {
            style.color = caret;
            style.selection_color = theme.accent.with_alpha(0.3);
        }
    }
}

/// Show/hide and color each checkbox's mark from its `Checked` state.
pub(super) fn update_glass_checkmarks(
    theme: Res<EditorTheme>,
    q_boxes: Query<(Has<Checked>, &Children), With<Checkbox>>,
    mut q_marks: Query<
        (&mut Node, &mut BackgroundColor),
        With<GlassCheckMark>,
    >,
) {
    for (checked, children) in &q_boxes {
        for child in children.iter() {
            if let Ok((mut node, mut bg)) = q_marks.get_mut(child) {
                node.display = if checked {
                    Display::Flex
                } else {
                    Display::None
                };
                bg.0 = theme.accent;
            }
        }
    }
}

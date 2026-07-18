//! Glass *widget* builders (spawn a whole thing). Styling for a node
//! you already built is the [`Glass`] marker component itself
//! (`Glass::Panel`, `Glass::tab(active)`, ...), not a builder here.

use bevy::feathers::cursor::EntityCursor;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui::Checked;
use bevy::ui_widgets::{Button, Checkbox};
use bevy::window::SystemCursorIcon;

use super::preset::Glass;
use crate::ui::theme::EditorTheme;

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

/// Marks a [`glass_field`] text input, for cursor theming.
#[derive(Component, Default, Clone)]
pub(super) struct GlassField;

/// A glass text input on the headless [`EditableText`]. Read its
/// contents via `EditableText::value()`; observe
/// [`TextEditChange`](bevy::text::TextEditChange) for edits.
pub fn glass_field() -> impl Scene {
    bsn! {
        GlassField
        EditableText { cursor_width: 0.3 }
        template_value(Glass::Field)
        TextLayout { linebreak: LineBreak::NoWrap }
        TextFont { font_size: FontSize::Px(12.0) }
        TextCursorStyle::default()
        TabIndex(0)
        EntityCursor::System(SystemCursorIcon::Text)
        Node {
            min_width: Val::Px(80.0),
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
        }
    }
}

/// Theme each text field's caret/selection from the accent color.
pub(super) fn update_glass_field_cursors(
    theme: Res<EditorTheme>,
    mut q: Query<&mut TextCursorStyle, With<GlassField>>,
) {
    for mut style in &mut q {
        if style.color != theme.accent {
            style.color = theme.accent;
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

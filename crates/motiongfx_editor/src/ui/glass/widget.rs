//! Named glass widget/component builders, so call sites don't spell
//! out [`Glass`] variants. Each surface builder returns the marker
//! [`Glass`] component (a `Bundle`), usable in `bsn!`
//! (`template_value(glass::panel())`) and world spawns alike.

use bevy::feathers::cursor::EntityCursor;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;
use bevy::window::SystemCursorIcon;

use super::preset::{Glass, GlassifyChild};

/// Large content surface (panels, viewports, name column).
pub fn panel() -> Glass {
    Glass::Panel
}

/// Tab-bar / chrome strip.
pub fn bar() -> Glass {
    Glass::Bar
}

/// Floating menu, dense enough for text.
pub fn popup() -> Glass {
    Glass::Popup
}

/// Soft drop-target feedback rect.
pub fn overlay() -> Glass {
    Glass::Overlay
}

/// Recessed input fill (text field, checkbox box).
pub fn field() -> Glass {
    Glass::Field
}

/// A tab pill in its active (`true`) or invisible-idle (`false`)
/// state.
pub fn tab(active: bool) -> Glass {
    if active {
        Glass::TabActive
    } else {
        Glass::TabIdle
    }
}

/// The faint hovered-tab pill.
pub fn tab_hover() -> Glass {
    Glass::TabHover
}

/// Glass a widget's first child once it exists — for backgrounds that
/// live on a scene-spawned child we can't reach synchronously.
pub fn glassify_child(glass: Glass) -> GlassifyChild {
    GlassifyChild(glass)
}

/// A glass-styled button carrying the headless [`Button`] behavior;
/// drop-in replacement for `themed_button`. Emits
/// [`bevy::ui_widgets::Activate`].
pub fn button<M>(width: f32, height: f32) -> impl Scene
where
    M: Component + Default + Unpin + Clone,
{
    bsn! {
        M
        template_value(Glass::Button)
        Button
        Hovered
        Node {
            width: Val::Px(width),
            height: Val::Px(height),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
        }
        EntityCursor::System(SystemCursorIcon::Pointer)
    }
}

use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui::widget::ImageNode;
use bevy::window::SystemCursorIcon;

/// A chromeless icon-only button: no background/material of its own
/// (unlike [`crate::glass::glass_button`]), just a centered icon in a
/// square hit area. Callers append their own click observer and, for
/// identification, their own marker.
#[derive(SceneComponent, Default, Clone)]
#[scene(GhostButtonProps)]
pub struct GhostButton;

pub struct GhostButtonProps {
    /// Asset path of the icon.
    pub icon: String,
    pub color: Color,
    /// Outer hit-area size.
    pub size: Val,
    pub icon_size: Val,
    pub radius: Val,
}

impl Default for GhostButtonProps {
    fn default() -> Self {
        Self {
            icon: String::new(),
            color: Color::WHITE,
            size: Val::Px(18.0),
            icon_size: Val::Px(11.0),
            radius: Val::ZERO,
        }
    }
}

impl GhostButton {
    fn scene(props: GhostButtonProps) -> impl Scene {
        let GhostButtonProps {
            icon,
            color,
            size,
            icon_size,
            radius,
        } = props;
        bsn! {
            GhostButton
            EntityCursor::System(SystemCursorIcon::Pointer)
            Node {
                width: size,
                height: size,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(radius),
            }
            Children [(
                ImageNode {
                    image: {icon},
                    color: {color},
                }
                Node { width: icon_size, height: icon_size }
            )]
        }
    }
}

//! Reading back what the layout resolved to.

use bevy::prelude::*;
use bevy::ui::UiGlobalTransform;

/// A node's rect in logical (UI) coordinates, which is the space a
/// pointer's position arrives in.
pub fn logical_rect(
    computed: &ComputedNode,
    transform: &UiGlobalTransform,
) -> Rect {
    let inv = computed.inverse_scale_factor();
    let size = computed.size() * inv;
    let (_scale, _angle, center) =
        transform.to_scale_angle_translation();
    let center = center.trunc() * inv;

    Rect::from_center_size(center, size)
}

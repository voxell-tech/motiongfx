//! View-plumbing systems: retarget the composition's scene cameras to
//! the offscreen preview image, and fit that image above the panel.
//! (The name column's scroll-locking lives in `scene.rs`.)

use bevy::camera::RenderTarget;
use bevy::prelude::*;

use crate::scene::TrackViewportCamera;
use crate::{EditorSettings, PreviewImage};

/// Point every scene camera (all but the editor's own
/// [`TrackViewportCamera`]) at the offscreen [`PreviewImage`] instead
/// of the window. `bevy_ui` then scales that image to fit the preview
/// area, so growing the panel shrinks the whole composition
/// uniformly.
pub(crate) fn retarget_scene_cameras(
    mut commands: Commands,
    preview: Res<PreviewImage>,
    q_camera: Query<
        (Entity, Option<&RenderTarget>),
        (With<Camera>, Without<TrackViewportCamera>),
    >,
) {
    for (entity, current) in &q_camera {
        let done = matches!(
            current,
            Some(RenderTarget::Image(t)) if t.handle == preview.0,
        );
        if !done {
            commands.entity(entity).insert(RenderTarget::Image(
                preview.0.clone().into(),
            ));
        }
    }
}

/// Fit the preview into its parent area, preserving the composition's
/// aspect ratio (letterbox) so it never stretches.
pub(crate) fn preview_fit(
    world: &World,
    node: Entity,
) -> Option<(Val, Val)> {
    let area = world.get::<ChildOf>(node)?.parent();
    let computed = world.get::<ComputedNode>(area)?;
    let avail = computed.size() * computed.inverse_scale_factor();
    if avail.x <= 0.0 || avail.y <= 0.0 {
        return None;
    }

    let comp =
        world.resource::<EditorSettings>().physical_size.as_vec2();
    let aspect = comp.x / comp.y;
    let mut w = avail.x;
    let mut h = w / aspect;
    if h > avail.y {
        h = avail.y;
        w = h * aspect;
    }
    Some((Val::Px(w), Val::Px(h)))
}

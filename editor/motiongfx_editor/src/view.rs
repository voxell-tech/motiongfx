//! View-plumbing systems: retarget the composition's scene cameras to
//! the offscreen preview image, fit that image above the panel, and
//! keep the name column's scroll locked to the track.

use bevy::camera::RenderTarget;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;

use crate::scene::{
    NamePanel, PreviewArea, PreviewDisplay, TrackViewport,
    TrackViewportCamera,
};
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

/// Size the preview [`ImageNode`](bevy::ui::widget::ImageNode) to fit
/// the available area above the panel while preserving the
/// composition's aspect ratio (letterbox), so it never stretches.
pub(crate) fn fit_preview_image(
    settings: Res<EditorSettings>,
    q_area: Query<&ComputedNode, With<PreviewArea>>,
    mut q_display: Query<&mut Node, With<PreviewDisplay>>,
) {
    let Ok(area) = q_area.single() else {
        return;
    };
    let Ok(mut node) = q_display.single_mut() else {
        return;
    };

    let avail = area.size() * area.inverse_scale_factor();
    if avail.x <= 0.0 || avail.y <= 0.0 {
        return;
    }

    let comp = settings.physical_size.as_vec2();
    let aspect = comp.x / comp.y;
    let mut w = avail.x;
    let mut h = w / aspect;
    if h > avail.y {
        h = avail.y;
        w = h * aspect;
    }

    let (w, h) = (Val::Px(w), Val::Px(h));
    if node.width != w {
        node.width = w;
    }
    if node.height != h {
        node.height = h;
    }
}

/// Keep the name column's vertical scroll locked to the track
/// viewport.
pub(crate) fn sync_name_scroll(
    q_viewport: Query<&ScrollPosition, With<TrackViewport>>,
    mut q_name_panel: Query<
        &mut ScrollPosition,
        (With<NamePanel>, Without<TrackViewport>),
    >,
) {
    let Ok(viewport) = q_viewport.single() else {
        return;
    };
    let Ok(mut name_scroll) = q_name_panel.single_mut() else {
        return;
    };
    if name_scroll.y != viewport.y {
        name_scroll.y = viewport.y;
    }
}

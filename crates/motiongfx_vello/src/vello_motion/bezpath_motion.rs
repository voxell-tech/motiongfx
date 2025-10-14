use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_vello_renderer::vello::kurbo;
use motiongfx_bevy::prelude::*;
use motiongfx_core::{prelude::*, sequence::sequence_update_system};

use crate::{
    fill_style::FillStyleMotion,
    stroke_style::StrokeStyleMotion,
    vello_vector::{
        bezpath::{VelloBezPath, VelloBezPathBundle},
        VelloBuilder,
    },
};

pub(crate) struct VelloBezPathMotionPlugin;

impl Plugin for VelloBezPathMotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (sequence_update_system::<VelloBezPath, f32, EmptyRes>,),
        );
    }
}

pub struct VelloBezPathBundleMotion {
    pub path: VelloBezPathMotion,
    pub fill: FillStyleMotion,
    pub stroke: StrokeStyleMotion,
    pub transform: TransformMotion,
}

impl VelloBezPathBundleMotion {
    pub fn new(target_id: Entity, bundle: VelloBezPathBundle) -> Self {
        Self {
            path: VelloBezPathMotion::new(target_id),
            fill: FillStyleMotion::new(target_id, bundle.fill),
            stroke: StrokeStyleMotion::new(target_id, bundle.stroke),
            transform: TransformMotion::new(target_id, bundle.scene_bundle.transform),
        }
    }
}

pub struct VelloBezPathMotion {
    target_id: Entity,
    trace: f32,
}

impl VelloBezPathMotion {
    pub fn new(target_id: Entity) -> Self {
        Self {
            target_id,
            trace: 1.0,
        }
    }

    pub fn trace_to(&mut self, new_trace: f32) -> Action<VelloBezPath, f32, EmptyRes> {
        let action = Action::new(self.target_id, self.trace, new_trace, Self::trace_interp);

        self.trace = new_trace;
        action
    }

    fn trace_interp(
        vello_path: &mut VelloBezPath,
        begin: &f32,
        end: &f32,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        let origin_pathels = vello_path.origin_path.elements();
        let pathels = vello_path.path.elements_mut();

        let pathel_count = origin_pathels.len();
        let trace = (end - begin) * t + begin;
        let trace_raw = trace * pathel_count as f32;

        let mut most_recent_initial = kurbo::Point::new(0.0, 0.0);
        let mut most_recent_point = kurbo::Point::new(0.0, 0.0);

        let mut path_index = 0;
        for origin_pathel in origin_pathels {
            let mut interp_value = trace_raw - path_index as f32;

            if interp_value <= 0.0 {
                pathels[path_index] = kurbo::PathEl::MoveTo(kurbo::Point::default());
            } else {
                // Clamp value within 1.0
                interp_value = f32::min(interp_value, 1.0);
                match origin_pathel {
                    kurbo::PathEl::MoveTo(p) => {
                        pathels[path_index] = kurbo::PathEl::MoveTo(*p);

                        most_recent_initial = *p;
                        most_recent_point = *p;
                    }
                    kurbo::PathEl::LineTo(p) => {
                        pathels[path_index] =
                            interp_pathel(most_recent_point, *origin_pathel, interp_value);

                        most_recent_point = *p;
                    }
                    kurbo::PathEl::QuadTo(_, p) => {
                        pathels[path_index] =
                            interp_pathel(most_recent_point, *origin_pathel, interp_value);

                        most_recent_point = *p;
                    }
                    kurbo::PathEl::CurveTo(.., p) => {
                        pathels[path_index] =
                            interp_pathel(most_recent_point, *origin_pathel, interp_value);

                        most_recent_point = *p;
                    }
                    kurbo::PathEl::ClosePath => {
                        if interp_value == 1.0 {
                            pathels[path_index] = kurbo::PathEl::ClosePath;
                        } else {
                            pathels[path_index] = interp_pathel(
                                most_recent_point,
                                kurbo::PathEl::MoveTo(most_recent_initial),
                                interp_value,
                            );
                        }
                    }
                }
            }

            path_index += 1;
        }

        vello_path.set_built(false);
    }
}

fn interp_pathel(p0: kurbo::Point, pathel: kurbo::PathEl, t: f32) -> kurbo::PathEl {
    if t == 1.0 {
        return pathel;
    }

    match pathel {
        kurbo::PathEl::MoveTo(p1) => kurbo::PathEl::MoveTo(kurbo::Point::lerp(p0, p1, t as f64)),
        kurbo::PathEl::LineTo(p1) => kurbo::PathEl::LineTo(kurbo::Point::lerp(p0, p1, t as f64)),
        kurbo::PathEl::QuadTo(p1, p2) => {
            let t = t as f64;
            // Point between p0 and p1
            let x0 = kurbo::Point::lerp(p0, p1, t);
            // Point between p1 and p2
            let x1 = kurbo::Point::lerp(p1, p2, t);
            // Point on curve
            let end_p = kurbo::Point::lerp(x0, x1, t);

            kurbo::PathEl::QuadTo(x0, end_p)
        }
        kurbo::PathEl::CurveTo(p1, p2, p3) => {
            let t = t as f64;
            // Point between p0 and p1
            let x0 = kurbo::Point::lerp(p0, p1, t);
            // Point between p1 and p2
            let x1 = kurbo::Point::lerp(p1, p2, t);
            // Point between p2 and p3
            let x2 = kurbo::Point::lerp(p2, p3, t);
            // Point between x0 and x1
            let y0 = kurbo::Point::lerp(x0, x1, t);
            // Point between x1 and x2
            let y1 = kurbo::Point::lerp(x1, x2, t);
            // Point on curve
            let end_p = kurbo::Point::lerp(y0, y1, t);

            kurbo::PathEl::CurveTo(x0, y0, end_p)
        }
        kurbo::PathEl::ClosePath => kurbo::PathEl::ClosePath,
    }
}

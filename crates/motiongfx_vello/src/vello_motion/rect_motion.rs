use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::DVec2;
use bevy_vello_renderer::vello::kurbo;
use motiongfx_bevy::prelude::*;
use motiongfx_core::{prelude::*, sequence::sequence_update_system};

use crate::{
    fill_style::FillStyleMotion,
    stroke_style::StrokeStyleMotion,
    vello_vector::{
        rect::{VelloRect, VelloRectBundle},
        VelloBuilder,
    },
};

pub(crate) struct VelloRectMotionPlugin;

impl Plugin for VelloRectMotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                sequence_update_system::<VelloRect, kurbo::Rect, EmptyRes>,
                sequence_update_system::<VelloRect, f64, EmptyRes>,
                sequence_update_system::<VelloRect, kurbo::RoundedRectRadii, EmptyRes>,
            ),
        );
    }
}

pub struct VelloRectBundleMotion {
    pub rect: VelloRectMotion,
    pub fill: FillStyleMotion,
    pub stroke: StrokeStyleMotion,
    pub transform: TransformMotion,
}

impl VelloRectBundleMotion {
    pub fn new(target_id: Entity, bundle: VelloRectBundle) -> Self {
        Self {
            rect: VelloRectMotion::new(target_id, bundle.rect),
            fill: FillStyleMotion::new(target_id, bundle.fill),
            stroke: StrokeStyleMotion::new(target_id, bundle.stroke),
            transform: TransformMotion::new(target_id, bundle.scene_bundle.transform),
        }
    }
}

pub struct VelloRectMotion {
    target_id: Entity,
    vello_rect: VelloRect,
}

impl VelloRectMotion {
    pub fn new(target_id: Entity, vello_rect: VelloRect) -> Self {
        Self {
            target_id,
            vello_rect,
        }
    }

    // =====================
    // Rect
    // =====================
    pub fn inflate(
        &mut self,
        inflation: impl Into<DVec2>,
    ) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        let inflation: DVec2 = inflation.into();

        let new_rect: kurbo::Rect = self.vello_rect.rect.inflate(inflation.x, inflation.y);

        let action: Action<VelloRect, kurbo::Rect, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect,
            new_rect,
            Self::rect_interp,
        );

        self.vello_rect.rect = new_rect;

        action
    }

    pub fn rect_to(
        &mut self,
        new_rect: impl Into<kurbo::Rect>,
    ) -> Action<VelloRect, kurbo::Rect, EmptyRes> {
        let new_rect: kurbo::Rect = new_rect.into();

        let action: Action<VelloRect, kurbo::Rect, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect,
            new_rect,
            Self::rect_interp,
        );

        self.vello_rect.rect = new_rect;

        action
    }

    fn rect_interp(
        vello_rect: &mut VelloRect,
        begin: &kurbo::Rect,
        end: &kurbo::Rect,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect = kurbo::Rect::lerp(begin, end, t);
        vello_rect.set_built(false);
    }

    // =====================
    // Rect.x0
    // =====================
    /// Expand the left side of the rect.
    pub fn expand_left(&mut self, expansion: f64) -> Action<VelloRect, f64, EmptyRes> {
        let new_x0: f64 = self.vello_rect.rect.x0 - expansion;

        let action: Action<VelloRect, f64, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect.x0,
            new_x0,
            Self::rect_x0_interp,
        );

        self.vello_rect.rect.x0 = new_x0;

        action
    }

    fn rect_x0_interp(
        vello_rect: &mut VelloRect,
        begin: &f64,
        end: &f64,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect.x0 = f64::lerp(begin, end, t);
        vello_rect.set_built(false);
    }

    // =====================
    // Rect.x1
    // =====================
    /// Expand the right side of the rect.
    pub fn expand_right(&mut self, expansion: f64) -> Action<VelloRect, f64, EmptyRes> {
        let new_x1: f64 = self.vello_rect.rect.x1 + expansion;

        let action: Action<VelloRect, f64, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect.x1,
            new_x1,
            Self::rect_x1_interp,
        );

        self.vello_rect.rect.x1 = new_x1;

        action
    }

    fn rect_x1_interp(
        vello_rect: &mut VelloRect,
        begin: &f64,
        end: &f64,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect.x1 = f64::lerp(begin, end, t);
        vello_rect.set_built(false);
    }

    // =====================
    // Rect.y0
    // =====================
    /// Expand the bottom side of the rect.
    pub fn expand_bottom(&mut self, expansion: f64) -> Action<VelloRect, f64, EmptyRes> {
        let new_y0: f64 = self.vello_rect.rect.y0 - expansion;

        let action: Action<VelloRect, f64, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect.y0,
            new_y0,
            Self::rect_y0_interp,
        );

        self.vello_rect.rect.y0 = new_y0;

        action
    }

    fn rect_y0_interp(
        vello_rect: &mut VelloRect,
        begin: &f64,
        end: &f64,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect.y0 = f64::lerp(begin, end, t);
        vello_rect.set_built(false);
    }

    // =====================
    // Rect.y1
    // =====================
    /// Expand the top side of the rect.
    pub fn expand_top(&mut self, expansion: f64) -> Action<VelloRect, f64, EmptyRes> {
        let new_y1: f64 = self.vello_rect.rect.y1 + expansion;

        let action: Action<VelloRect, f64, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.rect.y1,
            new_y1,
            Self::rect_y1_interp,
        );

        self.vello_rect.rect.y1 = new_y1;

        action
    }

    fn rect_y1_interp(
        vello_rect: &mut VelloRect,
        begin: &f64,
        end: &f64,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.rect.y1 = f64::lerp(begin, end, t);
        vello_rect.set_built(false);
    }

    // =====================
    // Radii
    // =====================
    pub fn radii_to(
        &mut self,
        new_radii: impl Into<kurbo::RoundedRectRadii>,
    ) -> Action<VelloRect, kurbo::RoundedRectRadii, EmptyRes> {
        let new_radii: kurbo::RoundedRectRadii = new_radii.into();

        let action: Action<VelloRect, kurbo::RoundedRectRadii, EmptyRes> = Action::new(
            self.target_id,
            self.vello_rect.radii,
            new_radii,
            Self::radii_interp,
        );

        self.vello_rect.radii = new_radii;

        action
    }

    fn radii_interp(
        vello_rect: &mut VelloRect,
        begin: &kurbo::RoundedRectRadii,
        end: &kurbo::RoundedRectRadii,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_rect.radii = kurbo::RoundedRectRadii::lerp(begin, end, t);
        vello_rect.set_built(false);
    }
}

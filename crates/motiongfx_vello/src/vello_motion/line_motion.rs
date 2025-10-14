use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::DVec2;
use bevy_vello_renderer::vello::kurbo;
use motiongfx_bevy::prelude::TransformMotion;
use motiongfx_core::{prelude::*, sequence::sequence_update_system};

use crate::{
    prelude::StrokeStyleMotion,
    vello_vector::{
        line::{VelloLine, VelloLineBundle},
        VelloBuilder,
    },
};

pub(crate) struct VelloLineMotionPlugin;

impl Plugin for VelloLineMotionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                sequence_update_system::<VelloLine, kurbo::Line, EmptyRes>,
                sequence_update_system::<VelloLine, kurbo::Point, EmptyRes>,
            ),
        );
    }
}

pub struct VelloLineBundleMotion {
    pub line: VelloLineMotion,
    pub stroke: StrokeStyleMotion,
    pub transform: TransformMotion,
}

impl VelloLineBundleMotion {
    pub fn new(target_id: Entity, bundle: VelloLineBundle) -> Self {
        Self {
            line: VelloLineMotion::new(target_id, bundle.line),
            stroke: StrokeStyleMotion::new(target_id, bundle.stroke),
            transform: TransformMotion::new(target_id, bundle.scene_bundle.transform),
        }
    }
}

pub struct VelloLineMotion {
    target_id: Entity,
    vello_line: VelloLine,
}

impl VelloLineMotion {
    pub fn new(target_id: Entity, vello_line: VelloLine) -> Self {
        Self {
            target_id,
            vello_line,
        }
    }

    pub fn line_to(
        &mut self,
        new_line: impl Into<kurbo::Line>,
    ) -> Action<VelloLine, kurbo::Line, EmptyRes> {
        let new_line: kurbo::Line = new_line.into();

        let action: Action<VelloLine, kurbo::Line, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line,
            new_line,
            Self::line_interp,
        );

        self.vello_line.line = new_line;

        action
    }

    /// Extend the line based on given `DVec2` percentage where `x` represents `p0` and `y` represents `p1`.
    pub fn percentage_extend(
        &mut self,
        extension: f64,
        percentage: impl Into<DVec2>,
    ) -> Action<VelloLine, kurbo::Line, EmptyRes> {
        let percentage: DVec2 = percentage.into();

        let mut new_line: kurbo::Line = self.vello_line.line;
        let direction: kurbo::Vec2 = self.get_p0_direction();

        new_line.p0 += direction * extension * percentage.x;
        new_line.p1 -= direction * extension * percentage.y;

        let action: Action<VelloLine, kurbo::Line, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line,
            new_line,
            Self::line_interp,
        );

        self.vello_line.line = new_line;

        action
    }

    /// Extend the line on both `p0` and `p1`.
    pub fn extend(&mut self, extension: f64) -> Action<VelloLine, kurbo::Line, EmptyRes> {
        self.percentage_extend(extension, DVec2::new(1.0, 1.0))
    }

    fn line_interp(
        vello_line: &mut VelloLine,
        begin: &kurbo::Line,
        end: &kurbo::Line,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_line.line = kurbo::Line::lerp(begin, end, t);
        vello_line.set_built(false);
    }

    /// Extend the line only using `p0`.
    pub fn extend_p0(&mut self, extension: f64) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let direction: kurbo::Vec2 = self.get_p0_direction();
        let new_p0 = self.vello_line.line.p0 + direction * extension;

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p0,
            new_p0,
            Self::p0_interp,
        );

        self.vello_line.line.p0 = new_p0;

        action
    }

    /// Move `p0` to a new location.
    pub fn p0_to(&mut self, new_p0: impl Into<DVec2>) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let new_p0: DVec2 = new_p0.into();
        let new_p0: kurbo::Point = kurbo::Point::new(new_p0.x, new_p0.y);

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p0,
            new_p0,
            Self::p0_interp,
        );

        self.vello_line.line.p0 = new_p0;

        action
    }

    pub fn p0_add(
        &mut self,
        translation: impl Into<DVec2>,
    ) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let translation: DVec2 = translation.into();
        let translation: kurbo::Vec2 = kurbo::Vec2::new(translation.x, translation.y);

        let new_p0: kurbo::Point = self.vello_line.line.p0 + translation;

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p0,
            new_p0,
            Self::p0_interp,
        );

        self.vello_line.line.p0 = new_p0;

        action
    }

    fn p0_interp(
        vello_line: &mut VelloLine,
        begin: &kurbo::Point,
        end: &kurbo::Point,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_line.line.p0 = kurbo::Point::lerp(*begin, *end, t as f64);
        vello_line.set_built(false);
    }

    /// Extend the line only using `p1`.
    pub fn extend_p1(&mut self, extension: f64) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let direction: kurbo::Vec2 = self.get_p0_direction();
        let new_p1 = self.vello_line.line.p1 - direction * extension;

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p1,
            new_p1,
            Self::p1_interp,
        );

        self.vello_line.line.p1 = new_p1;

        action
    }

    /// Move `p1` to a new location.
    pub fn p1_to(&mut self, new_p1: impl Into<DVec2>) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let new_p1: DVec2 = new_p1.into();
        let new_p1: kurbo::Point = kurbo::Point::new(new_p1.x, new_p1.y);

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p1,
            new_p1,
            Self::p1_interp,
        );

        self.vello_line.line.p1 = new_p1;

        action
    }

    pub fn p1_add(
        &mut self,
        translation: impl Into<DVec2>,
    ) -> Action<VelloLine, kurbo::Point, EmptyRes> {
        let translation: DVec2 = translation.into();
        let translation: kurbo::Vec2 = kurbo::Vec2::new(translation.x, translation.y);

        let new_p1: kurbo::Point = self.vello_line.line.p1 + translation;

        let action: Action<VelloLine, kurbo::Point, EmptyRes> = Action::new(
            self.target_id,
            self.vello_line.line.p1,
            new_p1,
            Self::p1_interp,
        );

        self.vello_line.line.p1 = new_p1;

        action
    }

    fn p1_interp(
        vello_line: &mut VelloLine,
        begin: &kurbo::Point,
        end: &kurbo::Point,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        vello_line.line.p1 = kurbo::Point::lerp(*begin, *end, t as f64);
        vello_line.set_built(false);
    }

    fn get_p0_direction(&self) -> kurbo::Vec2 {
        let mut direction: kurbo::Vec2 = self.vello_line.line.p0 - self.vello_line.line.p1;

        // If direction has no magnitude, a horizontal direction will be assigned
        if direction.length_squared() == 0.0 {
            direction.x = -1.0
        }

        (direction).normalize()
    }
}

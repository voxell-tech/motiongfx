use motiongfx::motion_paths;

#[cfg(feature = "color")]
use bevy_color::{LinearRgba, Srgba};
use bevy_math::{Rect, Vec2, Vec3, Quat};
#[cfg(feature = "transform")]
use bevy_transform::prelude::Transform;

motion_paths! {
    Vec2 {
        x: f32,
        y: f32,
    }

    Vec3 {
        x: f32,
        y: f32,
        z: f32,
    }

    Quat {
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    }
}

#[cfg(feature = "color")]
motion_paths! {
    Srgba {
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32,
    }

    LinearRgba {
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32,
    }
}

motion_paths! {
    Rect {
        min: bevy_math::Vec2,
        max: bevy_math::Vec2,
    }
}

#[cfg(feature = "transform")]
motion_paths! {
    Transform {
        translation: bevy_math::Vec3,
        rotation: bevy_math::Quat,
        scale: bevy_math::Vec3,
    }
}

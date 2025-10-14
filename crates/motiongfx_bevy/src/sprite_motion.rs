use bevy_ecs::prelude::*;
use bevy_render::prelude::*;
use bevy_sprite::prelude::*;
use motiongfx_core::prelude::*;

pub struct SpriteMotion {
    target_id: Entity,
    sprite: Sprite,
}

impl SpriteMotion {
    pub fn new(target_id: Entity, sprite: Sprite) -> Self {
        Self { target_id, sprite }
    }

    pub fn color_to(&mut self, new_color: Color) -> Action<Sprite, Color, EmptyRes> {
        let action: Action<Sprite, Color, EmptyRes> = Action::new(
            self.target_id,
            self.sprite.color,
            new_color,
            Self::color_interp,
        );

        action
    }

    fn color_interp(
        sprite: &mut Sprite,
        begin: &Color,
        end: &Color,
        t: f32,
        _: &mut ResMut<EmptyRes>,
    ) {
        sprite.color = Color::lerp(begin, end, t);
    }

    pub fn alpha_to(&mut self, new_alpha: f32) -> Action<Sprite, f32, EmptyRes> {
        let action: Action<Sprite, f32, EmptyRes> = Action::new(
            self.target_id,
            self.sprite.color.a(),
            new_alpha,
            Self::alpha_interp,
        );

        action
    }

    fn alpha_interp(sprite: &mut Sprite, begin: &f32, end: &f32, t: f32, _: &mut ResMut<EmptyRes>) {
        sprite.color = sprite.color.with_a(f32::lerp(begin, end, t));
    }
}

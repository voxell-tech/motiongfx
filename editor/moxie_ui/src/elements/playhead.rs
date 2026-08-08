use bevy::prelude::*;

const PLAYHEAD_COLOR: Color = Color::srgb(0.95, 0.30, 0.35);

/// The playhead line, positioned by the editor's playhead system.
#[derive(SceneComponent, Default, Clone)]
#[scene(PlayheadLineProps)]
pub struct PlayheadLine;

#[derive(Default)]
pub struct PlayheadLineProps {
    pub left: f32,
}

impl PlayheadLine {
    fn scene(
        PlayheadLineProps { left }: PlayheadLineProps,
    ) -> impl Scene {
        bsn! {
            PlayheadLine
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                bottom: px(0),
                left: px(left),
                width: px(2),
            }
            ZIndex(10)
            BackgroundColor(PLAYHEAD_COLOR)
        }
    }
}

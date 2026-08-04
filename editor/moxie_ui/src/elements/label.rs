use bevy::feathers::theme::ThemedText;
use bevy::prelude::*;

/// A theme-inheriting text label.
#[derive(SceneComponent, Default, Clone)]
#[scene(LabelProps)]
pub struct Label;

#[derive(Default)]
pub struct LabelProps {
    pub text: String,
}

impl Label {
    fn scene(LabelProps { text }: LabelProps) -> impl Scene {
        bsn! {
            Label
            Text({text})
            ThemedText
            TextFont {
                font_size: FontSize::Px(13.0)
            }
        }
    }
}

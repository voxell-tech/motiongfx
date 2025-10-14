use bevy::color::{Color, Srgba};
use bevy::platform::collections::HashMap;
use core::hash::Hash;

#[derive(Clone)]
pub struct ColorPalette<Key: Sized> {
    palette: HashMap<Key, Color>,
}

impl<Key> ColorPalette<Key>
where
    Key: PartialEq + Eq + Hash,
{
    /// Creates a new [`ColorPalette<Key>`].
    pub fn new(palette: HashMap<Key, Color>) -> Self {
        Self { palette }
    }

    pub fn insert(&mut self, key: Key, value: Color) {
        self.palette.insert(key, value);
    }

    pub fn get(&self, key: Key) -> Color {
        *self.palette.get(&key).unwrap()
    }
}

impl<Key> From<HashMap<Key, Color>> for ColorPalette<Key> {
    fn from(palette: HashMap<Key, Color>) -> Self {
        Self { palette }
    }
}

impl Default for ColorPalette<ColorKey> {
    /// Monokai Pro as the default `ColorPalatte`.
    fn default() -> Self {
        Self::new(
            [
                // Primary colors
                (ColorKey::Red, Srgba::hex("FF6188").unwrap().into()),
                (
                    ColorKey::Orange,
                    Srgba::hex("FC9867").unwrap().into(),
                ),
                (
                    ColorKey::Yellow,
                    Srgba::hex("FFD866").unwrap().into(),
                ),
                (
                    ColorKey::Green,
                    Srgba::hex("A9DC76").unwrap().into(),
                ),
                (
                    ColorKey::Blue,
                    Srgba::hex("78DCE8").unwrap().into(),
                ),
                (
                    ColorKey::Purple,
                    Srgba::hex("AB9DF2").unwrap().into(),
                ),
                // Base colors, sorted from darkest to lightest
                (
                    ColorKey::Base0,
                    Srgba::hex("19181A").unwrap().into(),
                ),
                (
                    ColorKey::Base1,
                    Srgba::hex("221F22").unwrap().into(),
                ),
                (
                    ColorKey::Base2,
                    Srgba::hex("2D2A2E").unwrap().into(),
                ),
                (
                    ColorKey::Base3,
                    Srgba::hex("403E41").unwrap().into(),
                ),
                (
                    ColorKey::Base4,
                    Srgba::hex("5B595C").unwrap().into(),
                ),
                (
                    ColorKey::Base5,
                    Srgba::hex("727072").unwrap().into(),
                ),
                (
                    ColorKey::Base6,
                    Srgba::hex("939293").unwrap().into(),
                ),
                (
                    ColorKey::Base7,
                    Srgba::hex("C1C0C0").unwrap().into(),
                ),
                (
                    ColorKey::Base8,
                    Srgba::hex("FCFCFA").unwrap().into(),
                ),
            ]
            .into(),
        )
    }
}

#[derive(Default, PartialEq, Eq, Hash)]
pub enum ColorKey {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
    Base0,
    Base1,
    Base2,
    Base3,
    Base4,
    Base5,
    Base6,
    Base7,
    #[default]
    Base8,
}

impl ColorKey {
    pub fn darkest() -> Self {
        Self::Base0
    }

    pub fn lightest() -> Self {
        Self::Base8
    }
}

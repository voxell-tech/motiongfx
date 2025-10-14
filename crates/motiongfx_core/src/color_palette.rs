use bevy_render::color::Color;
use bevy_utils::HashMap;
use core::hash::Hash;

#[derive(Clone)]
pub struct ColorPalette<Key: Sized> {
    default: Color,
    palette: HashMap<Key, Color>,
}

impl<Key> ColorPalette<Key>
where
    Key: PartialEq + Eq + Hash,
{
    /// Creates a new [`ColorPalette<Key>`].
    pub fn new() -> Self {
        Self {
            default: Color::default(),
            palette: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: Key, value: Color) {
        self.palette.insert(key, value);
    }

    pub fn get(&self, key: &Key) -> Option<&Color> {
        self.palette.get(key)
    }

    pub fn get_or_default(&self, key: &Key) -> &Color {
        self.palette.get(key).unwrap_or(&self.default)
    }
}

impl<Key> From<HashMap<Key, Color>> for ColorPalette<Key> {
    fn from(palette: HashMap<Key, Color>) -> Self {
        Self {
            default: Color::default(),
            palette,
        }
    }
}

impl Default for ColorPalette<ColorKey> {
    /// Monokai Pro as the default `ColorPalatte`.
    fn default() -> Self {
        Self {
            default: Color::default(),
            palette: [
                // Primary colors
                (ColorKey::Red, Color::hex("FF6188").unwrap()),
                (ColorKey::Orange, Color::hex("FC9867").unwrap()),
                (ColorKey::Yellow, Color::hex("FFD866").unwrap()),
                (ColorKey::Green, Color::hex("A9DC76").unwrap()),
                (ColorKey::Blue, Color::hex("78DCE8").unwrap()),
                (ColorKey::Purple, Color::hex("AB9DF2").unwrap()),
                // Base colors, sorted from darkest to lightest
                (ColorKey::Base0, Color::hex("19181A").unwrap()),
                (ColorKey::Base1, Color::hex("221F22").unwrap()),
                (ColorKey::Base2, Color::hex("2D2A2E").unwrap()),
                (ColorKey::Base3, Color::hex("403E41").unwrap()),
                (ColorKey::Base4, Color::hex("5B595C").unwrap()),
                (ColorKey::Base5, Color::hex("727072").unwrap()),
                (ColorKey::Base6, Color::hex("939293").unwrap()),
                (ColorKey::Base7, Color::hex("C1C0C0").unwrap()),
                (ColorKey::Base8, Color::hex("FCFCFA").unwrap()),
            ]
            .into(),
        }
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

//! The state an element starts in, before a style or a call site has
//! had its say.

use fynix_mock::OverrideDefault;

#[derive(Debug, Default, PartialEq)]
pub struct Font {
    pub size: u32,
    pub weight: u32,
}

/// Nothing overridden: the same as `#[derive(Default)]`.
#[derive(Debug, OverrideDefault, PartialEq)]
pub struct Plain {
    pub font: Font,
    pub text: String,
}

#[derive(Debug, OverrideDefault, PartialEq)]
pub struct Label {
    /// A value, whole.
    #[default(Font { size: 13, weight: 400 })]
    pub font: Font,
    #[default(String::from("Save"))]
    pub text: String,
}

#[derive(Debug, OverrideDefault, PartialEq)]
pub struct Title {
    /// The field's own default, with one of its fields moved.
    #[default(size: 24)]
    pub font: Font,
    pub text: String,
}

#[test]
fn an_untouched_field_uses_its_own_default() {
    assert_eq!(
        Plain::default(),
        Plain {
            font: Font { size: 0, weight: 0 },
            text: String::new(),
        }
    );
}

#[test]
fn a_value_replaces_the_default() {
    assert_eq!(
        Label::default(),
        Label {
            font: Font {
                size: 13,
                weight: 400
            },
            text: "Save".into(),
        }
    );
}

#[test]
fn overrides_keep_the_rest_of_the_default() {
    let title = Title::default();

    assert_eq!(title.font.size, 24, "overridden");
    assert_eq!(title.font.weight, 0, "still `Font`'s own default");
    assert_eq!(title.text, "", "no attribute at all");
}

#[test]
fn overrides_stack_within_one_field() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Heading {
        #[default(size: 32, weight: 700)]
        font: Font,
    }

    assert_eq!(
        Heading::default().font,
        Font {
            size: 32,
            weight: 700
        }
    );
}

#[test]
fn a_nested_element_starts_from_its_own_default() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Card {
        /// `Label` already defaults its font to 13; this moves only
        /// the size, and leaves the text `Label` chose.
        #[default(font: Font { size: 24, weight: 400 })]
        title: Label,
        padding: u32,
    }

    let card = Card::default();

    assert_eq!(card.title.font.size, 24);
    assert_eq!(card.title.text, "Save", "`Label`'s default text");
    assert_eq!(card.padding, 0);
}

#[test]
fn a_generic_element_defaults_every_argument() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Themed<T> {
        #[default(size: 18)]
        font: Font,
        look: T,
    }

    assert_eq!(
        Themed::<u32>::default(),
        Themed {
            font: Font {
                size: 18,
                weight: 0
            },
            look: 0,
        }
    );
}

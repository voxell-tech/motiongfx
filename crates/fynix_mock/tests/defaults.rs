//! Every way of writing `#[default(..)]`, one at a time.
//!
//! Nothing here is an element: the derive is about a struct's starting
//! state, and knows nothing of UI.

use fynix_mock::OverrideDefault;

/// A plain `Default`, to override fields of.
#[derive(Debug, Default, PartialEq)]
pub struct Font {
    pub size: u32,
    pub weight: u32,
}

/// A tuple struct, to reach by position.
#[derive(Debug, OverrideDefault, PartialEq)]
pub struct Padding(#[default(4)] pub u32, pub u32);

/// An enum, to name variants of.
#[derive(Debug, OverrideDefault, PartialEq)]
pub enum Weight {
    Thin,
    #[default]
    Regular,
    Exactly(u32),
    Named {
        size: u32,
    },
}

// What the field says nothing about.

#[test]
fn no_attribute_keeps_the_field_s_own_default() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Plain {
        font: Font,
        text: String,
    }

    assert_eq!(
        Plain::default(),
        Plain {
            font: Font { size: 0, weight: 0 },
            text: String::new(),
        }
    );
}

// A value, whole.

#[test]
fn expression_is_the_value() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Ruled {
        #[default(13)]
        size: u32,
        #[default(String::from("Save"))]
        text: String,
        #[default(Font { size: 1, weight: 2 })]
        font: Font,
    }

    assert_eq!(
        Ruled::default(),
        Ruled {
            size: 13,
            text: "Save".into(),
            font: Font { size: 1, weight: 2 },
        }
    );
}

#[test]
fn braces_say_a_value_was_meant() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Turn {
        /// An absolute path, which a leading `::` would otherwise
        /// read as a variant of `f32`.
        #[default({ ::core::f32::consts::PI })]
        angle: f32,
    }

    assert_eq!(Turn::default().angle, core::f32::consts::PI);
}

// Fields of the field, by name.

#[test]
fn named_overrides_keep_the_rest_of_the_default() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Title {
        #[default(size: 24)]
        font: Font,
    }

    let title = Title::default();

    assert_eq!(title.font.size, 24, "overridden");
    assert_eq!(title.font.weight, 0, "still `Font`'s own");
}

#[test]
fn named_overrides_stack_within_one_field() {
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
fn named_overrides_nest() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Label {
        #[default(size: 13)]
        font: Font,
        #[default(String::from("Label"))]
        text: String,
    }

    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Card {
        /// `Label` starts at its own default, and one field of that
        /// moves. The text it chose stays.
        #[default(font: Font { size: 24, weight: 400 })]
        title: Label,
    }

    let card = Card::default();

    assert_eq!(card.title.font.size, 24);
    assert_eq!(card.title.text, "Label", "`Label`'s own");
}

// Fields of the field, by position.

#[test]
fn index_overrides_name_a_position() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Frame {
        #[default(0: 1, 1: 2)]
        origin: (u32, u32),
        #[default(1: 8)]
        padding: Padding,
    }

    let frame = Frame::default();

    assert_eq!(frame.origin, (1, 2));
    assert_eq!(frame.padding, Padding(4, 8), "`Padding`'s own 4");
}

#[test]
fn pattern_overrides_the_position_it_marks() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Margin(#[default(_, 8, _)] (u32, u32, u32));

    assert_eq!(Margin::default().0, (0, 8, 0));
}

#[test]
fn rest_stands_for_the_positions_before() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Margin(#[default(.., 8)] (u32, u32, u32));

    assert_eq!(Margin::default().0, (0, 0, 8));
}

#[test]
fn rest_stands_for_the_positions_after() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Margin(#[default(8, ..)] (u32, u32, u32));

    assert_eq!(Margin::default().0, (8, 0, 0));
}

#[test]
fn rest_stands_for_the_positions_between() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Margin(#[default(.., 8, _)] (u32, u32, u32));

    assert_eq!(Margin::default().0, (0, 8, 0));
}

#[test]
fn pattern_reaches_a_tuple_struct_without_naming_it() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Frame {
        #[default(_, 8)]
        padding: Padding,
    }

    assert_eq!(Frame::default().padding, Padding(4, 8));
}

#[test]
fn position_holds_whatever_an_expression_does() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Spans {
        #[default(2 + 2, .., u32::from(3u8))]
        counts: (u32, u32, u32),
    }

    assert_eq!(Spans::default().counts, (4, 0, 3));
}

// Variants of the field's own type.

#[test]
fn leading_colon_names_a_unit_variant() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Heading {
        #[default(::Thin)]
        weight: Weight,
    }

    assert_eq!(Heading::default().weight, Weight::Thin);
}

#[test]
fn leading_colon_names_a_tuple_variant() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Heading {
        #[default(::Exactly(700))]
        weight: Weight,
    }

    assert_eq!(Heading::default().weight, Weight::Exactly(700));
}

#[test]
fn leading_colon_names_a_struct_variant() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Heading {
        #[default(::Named { size: 3 })]
        weight: Weight,
    }

    assert_eq!(Heading::default().weight, Weight::Named { size: 3 });
}

// Through an `Option`.

#[test]
fn rest_fills_an_option() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Badge {
        #[default(..)]
        font: Option<Font>,
        /// No attribute, so still nothing.
        text: Option<String>,
    }

    assert_eq!(
        Badge::default(),
        Badge {
            font: Some(Font { size: 0, weight: 0 }),
            text: None,
        }
    );
}

#[test]
fn overrides_through_an_option_mean_the_value_inside() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    struct Badge {
        #[default(size: 9)]
        font: Option<Font>,
        #[default(_, 8)]
        padding: Option<Padding>,
    }

    assert_eq!(
        Badge::default(),
        Badge {
            font: Some(Font { size: 9, weight: 0 }),
            padding: Some(Padding(4, 8)),
        }
    );
}

// What the derive can be written on.

#[test]
fn tuple_struct_defaults_by_position() {
    assert_eq!(Padding::default(), Padding(4, 0));
}

#[test]
fn enum_starts_in_the_marked_variant() {
    assert_eq!(Weight::default(), Weight::Regular);
    assert_ne!(Weight::default(), Weight::Thin);
}

#[test]
fn marked_struct_variant_defaults_its_fields() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    enum Fill {
        None,
        #[default]
        Solid {
            #[default(size: 2)]
            font: Font,
        },
    }

    assert_eq!(
        Fill::default(),
        Fill::Solid {
            font: Font { size: 2, weight: 0 }
        }
    );
    assert_ne!(Fill::default(), Fill::None);
}

#[test]
fn marked_tuple_variant_defaults_its_fields() {
    #[derive(Debug, OverrideDefault, PartialEq)]
    enum Edge {
        Square,
        #[default]
        Round(#[default(3)] u32),
    }

    assert_eq!(Edge::default(), Edge::Round(3));
    assert_ne!(Edge::default(), Edge::Square);
}

#[test]
fn generic_struct_defaults_every_argument() {
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

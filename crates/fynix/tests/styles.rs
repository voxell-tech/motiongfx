//! The three layers an element passes through before it is built: its
//! own base, a style, then the call site.
//!
//! A style is a mutation, so the order they run in is the precedence,
//! and nothing has to remember where a field's value came from.

use core::marker::PhantomData;

mod common;

use common::{FynixHost, Label, World};
use fynix::Fynix;
use fynix::elem;
use fynix::element::ElementBase;
use fynix::host::Host;
use fynix::style::Style;

/// Runs an [`elem!`]'s cascade with the mock theme.
fn create<E>(build: impl FnOnce(&()) -> E) -> E {
    build(&())
}

/// A style that writes `size`.
struct Title;

impl Style for Title {
    type Host = FynixHost;
    type Element = Label;

    fn apply(self, label: &mut Label, _theme: &()) {
        label.size = 10;
    }
}

#[test]
fn style_writes_over_the_default() {
    let label = create(elem!(!Title));

    assert_eq!(label.size, 10, "the style's");
    assert_eq!(label.text, "Label", "untouched, so the default's");
}

#[test]
fn fields_are_written_after_the_style() {
    let label = create(elem!(!Title, text = "Save"));

    assert_eq!(label.size, 10);
    assert_eq!(label.text, "Save");
}

#[test]
fn the_call_site_wins_over_the_style() {
    let label = create(elem!(!Title, size = 32u32));

    assert_eq!(label.size, 32, "the style set 10 first");
}

#[test]
fn the_style_can_be_any_expression() {
    struct Exactly(u32);

    impl Style for Exactly {
        type Host = FynixHost;
        type Element = Label;

        fn apply(self, label: &mut Label, _theme: &()) {
            label.size = self.0;
        }
    }

    // A literal and a call; the comma tells either from the fields.
    assert_eq!(create(elem!(!Exactly(7))).size, 7);
    assert_eq!(
        create(elem!(!Exactly(7), text = "Save")).text,
        "Save"
    );
}

#[test]
fn generic_elements_and_styles_both_carry_their_arguments() {
    #[derive(Default)]
    struct Themed<L> {
        size: u32,
        look: L,
    }

    impl<L: Default> ElementBase<FynixHost> for Themed<L> {
        fn base(_theme: &()) -> Self {
            Self::default()
        }
    }

    #[derive(Default, Debug, PartialEq)]
    struct Dark;

    struct Wide<L>(PhantomData<fn() -> L>);

    impl<L: Default> Style for Wide<L> {
        type Host = FynixHost;
        type Element = Themed<L>;

        fn apply(self, themed: &mut Themed<L>, _theme: &()) {
            themed.size = 10;
        }
    }

    // The element is a type, so its arguments are written as one.
    let themed = create(elem!(Themed<Dark>, size = 32u32));

    assert_eq!(themed.size, 32);
    assert_eq!(themed.look, Dark);

    // The style is an expression, so it takes a turbofish.
    let themed = create(elem!(!Wide::<Dark>(PhantomData)));

    assert_eq!(themed.size, 10);
}

#[test]
fn fields_alone_start_from_the_default() {
    let label = create(elem!(Label, text = "Save"));

    assert_eq!(label.text, "Save");
    assert_eq!(label.size, 13, "no style ran, so the default's");
}

#[test]
fn field_paths_reach_as_deep_as_they_go() {
    #[derive(Default)]
    struct Font {
        size: u32,
    }

    #[derive(Default)]
    struct Card {
        font: Font,
    }

    impl ElementBase<FynixHost> for Card {
        fn base(_theme: &()) -> Self {
            Self::default()
        }
    }

    struct Wide;

    impl Style for Wide {
        type Host = FynixHost;
        type Element = Card;

        fn apply(self, card: &mut Card, _theme: &()) {
            card.font.size = 1;
        }
    }

    let card = create(elem!(!Wide, font.size = 32u32));

    assert_eq!(card.font.size, 32);
}

#[test]
fn closures_are_there_for_what_fields_cannot_say() {
    let label = create(elem!(!Title, |label: &mut Label| {
        label.size = if label.text.is_empty() { 1 } else { 32 };
    }));

    assert_eq!(label.size, 32);
}

#[test]
fn the_kernel_builds_what_the_cascade_left() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new(());

    kernel.watch(
        root,
        |_| true,
        |ui| {
            ui.elem(elem!(!Title, text = "Save"));
        },
        &mut world,
    );

    let children = FynixHost::children(&world, root);
    assert_eq!(world.get(children[0]).text, "Save");
    assert_eq!(world.get(children[0]).size, 10);
}

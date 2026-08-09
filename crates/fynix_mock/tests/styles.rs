//! The three layers an element passes through before it is built: its
//! own default, a style, then the call site.
//!
//! A style is a mutation, so the order they run in is the precedence,
//! and nothing has to remember where a field's value came from.

use core::marker::PhantomData;

mod common;

use common::{Backend, Label, World};
use fynix_mock::Fynix;
use fynix_mock::elem;
use fynix_mock::element::Element;
use fynix_mock::host::Host;
use fynix_mock::store::Store;
use fynix_mock::style::{Raw, Style, StyledElem};

/// A style with something to say, so a test can see it run.
struct Title;

impl Style for Title {
    type Element = Label;

    fn apply(self, label: &mut Label) {
        label.size = 10;
    }
}

#[test]
fn style_writes_over_the_default() {
    let label = elem!(Title).create();

    assert_eq!(label.size, 10, "the style's");
    assert_eq!(label.text, "Label", "untouched, so the default's");
}

#[test]
fn fields_are_written_after_the_style() {
    let label = elem!(Title, { text = "Save" }).create();

    assert_eq!(label.size, 10);
    assert_eq!(label.text, "Save");
}

#[test]
fn the_call_site_wins_over_the_style() {
    let label = elem!(Title, { size = 32u32 }).create();

    assert_eq!(label.size, 32, "the style set 10 first");
}

#[test]
fn the_style_can_be_any_expression() {
    struct Exactly(u32);

    impl Style for Exactly {
        type Element = Label;

        fn apply(self, label: &mut Label) {
            label.size = self.0;
        }
    }

    // A literal, and a call. Both are told from the fields that
    // follow by the comma, so neither has to be a bare path.
    assert_eq!(elem!(Exactly(7)).create().size, 7);
    assert_eq!(
        elem!(Exactly(7), { text = "Save" }).create().text,
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

    #[derive(Default, Debug, PartialEq)]
    struct Dark;

    struct Wide<L>(PhantomData<fn() -> L>);

    impl<L: Default> Style for Wide<L> {
        type Element = Themed<L>;

        fn apply(self, themed: &mut Themed<L>) {
            themed.size = 10;
        }
    }

    // The element is a type, so its arguments are written as one.
    let themed = elem!(!Themed<Dark> { size = 32u32 }).create();

    assert_eq!(themed.size, 32);
    assert_eq!(themed.look, Dark);

    // The style is an expression, so it needs the turbofish that any
    // other expression would.
    let themed = elem!(Wide::<Dark>(PhantomData)).create();

    assert_eq!(themed.size, 10);
}

#[test]
fn fields_alone_start_from_the_default() {
    let label = elem!(!Label { text = "Save" }).create();

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

    struct Wide;

    impl Style for Wide {
        type Element = Card;

        fn apply(self, card: &mut Card) {
            card.font.size = 1;
        }
    }

    let card = elem!(Wide, { font.size = 32u32 }).create();

    assert_eq!(card.font.size, 32);
}

#[test]
fn closures_are_there_for_what_fields_cannot_say() {
    let label = elem!(Title, |label: &mut Label| {
        label.size = if label.text.is_empty() { 1 } else { 32 };
    })
    .create();

    assert_eq!(label.size, 32);
}

#[test]
fn every_case_is_the_same_argument() {
    fn create<S: StyledElem>(styled: S) -> S::Element {
        styled.create()
    }

    assert_eq!(create(elem!(Title)).size, 10);
    assert_eq!(create(elem!(Title, { size = 32u32 })).size, 32);
    assert_eq!(create(elem!(!Label { size = 32u32 })).size, 32);
    assert_eq!(
        create(Raw(Label {
            text: "Save".into(),
            size: 32,
        }))
        .size,
        32
    );
}

#[test]
fn finished_element_passes_through_untouched() {
    let label = Raw(Label {
        text: "Save".into(),
        size: 32,
    })
    .create();

    assert_eq!(label.text, "Save");
    assert_eq!(label.size, 32, "no default, and no style");
}

#[test]
fn what_the_cascade_left_is_what_gets_built() {
    let (mut world, parent) = World::with_root();
    let mut store = Store::new();

    let label = elem!(Title, { text = "Save" }).create();
    let node = Element::<Backend>::build(
        &label, &mut world, parent, &mut store,
    );

    assert_eq!(world.get(node).text, "Save");
    assert_eq!(world.get(node).size, 10);
}

#[test]
fn the_builder_takes_a_styled_element_whole() {
    let (mut world, root) = World::with_root();
    let mut kernel = Fynix::new();

    kernel.watch(
        root,
        |_: &World, _| true,
        |ui| {
            ui.elem(elem!(Title, { text = "Save" }));
        },
    );

    kernel.flush(&mut world);

    let children = Backend::children(&world, root);
    assert_eq!(world.get(children[0]).text, "Save");
    assert_eq!(world.get(children[0]).size, 10);
}

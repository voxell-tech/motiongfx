//! What makes a field id distinct.
//!
//! An id names a *path*, not a type. Two fields holding the same
//! `Label` are two different paths and get two different ids, which is
//! what lets one store hold both.
//!
//! Run with `cargo run -p fynix_mock --example field_ids`.

use fynix_mock::element::{Element, Fields};
use fynix_mock::lenz::{FieldId, Lenz};

#[derive(Lenz, Element)]
pub struct Label {
    pub text: String,
    pub size: u32,
}

/// Two children of the same type, side by side.
#[derive(Lenz, Element)]
pub struct Pair {
    #[elem]
    pub top: Label,
    #[elem]
    pub bottom: Label,
    pub gap: u32,
}

/// A different struct, with a field of the same name and type.
#[derive(Lenz, Element)]
pub struct Stack {
    #[elem]
    pub top: Label,
    pub gap: u32,
}

pub trait Look: 'static {}

pub struct Dark;
pub struct Light;

impl Look for Dark {}
impl Look for Light {}

/// Generic, so its paths differ per argument.
#[derive(Lenz, Element)]
pub struct Themed<L: Look> {
    #[elem]
    pub label: Label,
    pub look: L,
}

fn show(what: &str, id: FieldId) {
    println!("  {what:<28} {id:?}");
}

fn main() {
    println!("\ntwo fields of one type, in one struct");
    let top = Pair::path().top().ids()[0];
    let bottom = Pair::path().bottom().ids()[0];
    show("Pair::top", top);
    show("Pair::bottom", bottom);
    assert_ne!(top, bottom, "same type, but different paths");

    println!("\nthe same field name in another struct");
    let stack_top = Stack::path().top().ids()[0];
    show("Stack::top", stack_top);
    assert_ne!(top, stack_top, "same name and type, other owner");

    println!("\none generic struct, two arguments");
    let dark = Themed::<Dark>::path().label().ids()[0];
    let light = Themed::<Light>::path().label().ids()[0];
    show("Themed::<Dark>::label", dark);
    show("Themed::<Light>::label", light);
    assert_ne!(dark, light, "the marker carries the argument");

    println!("\nthe same walk, twice");
    show("Pair::top", Pair::path().top().ids()[0]);
    assert_eq!(top, Pair::path().top().ids()[0], "ids are stable");

    println!("\nthe same field, reached two ways");
    let walked = Pair::path().gap().ids()[0];
    let named = Pair::field_id(PairField::Gap);
    show("Pair::path().gap()", walked);
    show("PairField::Gap", named);
    assert_eq!(walked, named, "one field, one id");

    println!("\ntwo hops name two ids");
    let path = Pair::path().bottom().text();
    let ids = path.ids();
    show("Pair::bottom", ids[0]);
    show("  then Label::text", ids[1]);
    assert_eq!(ids[0], bottom, "the first hop is the child");

    // The second hop is `Label`'s own field, so `Label` recognises it
    // and `Pair` does not: a child patches itself.
    assert_eq!(Label::field(ids[1]), Some(LabelField::Text));
    assert!(Pair::field(ids[0]).is_none(), "`top` is an element");

    println!("\nall distinct.\n");
}

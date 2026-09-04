//! What makes a field id distinct.
//!
//! An id names a *path*, not a type. Two fields holding the same
//! `Label` are two different paths and get two different ids, which is
//! what lets one store hold both.
//!
//! Run with `cargo run -p fynix --example field_ids`.

use core::marker::PhantomData;

use fynix::element::{Fields, element};
use fynix::host::Host;
use fynix::lenz::FieldId;
use fynix::ui::{FieldPatch, Patch};

/// A stand-in backend: this example inspects field ids, never builds
/// anything, so nothing here is ever called.
pub struct FynixHost;

impl Host for FynixHost {
    type Node = usize;
    type World = ();
    type Theme = ();
    fn delta(_: &()) -> core::time::Duration {
        core::time::Duration::ZERO
    }
    fn spawn(_: &mut (), _: usize) -> usize {
        0
    }
    fn exists(_: &(), _: usize) -> bool {
        true
    }
    fn children(_: &(), _: usize) -> Vec<usize> {
        Vec::new()
    }
    fn despawn(_: &mut (), _: usize) {}
}

/// This example inspects field ids, never builds anything, so no tag
/// here ever writes.
pub struct Noop<T>(PhantomData<fn() -> T>);

impl<T: 'static> FieldPatch<FynixHost> for Noop<T> {
    type Target = T;

    fn patch(_: &mut Patch<FynixHost>, _: &T) {}
}

#[element]
pub struct Label {
    #[elem(patch = Noop::<String>)]
    pub text: String,
    #[elem(patch = Noop::<u32>)]
    pub size: u32,
}

/// Two children of the same type, side by side.
#[element]
pub struct Pair {
    #[elem(child)]
    pub top: Label,
    #[elem(child)]
    pub bottom: Label,
    #[elem(patch = Noop::<u32>)]
    pub gap: u32,
}

/// A different struct, with a field of the same name and type.
#[element]
pub struct Stack {
    #[elem(child)]
    pub top: Label,
    #[elem(patch = Noop::<u32>)]
    pub gap: u32,
}

/// Reached through two owners, to walk the same `Label` twice over.
#[element]
pub struct Card {
    #[elem(child)]
    pub header: Label,
}

#[element]
pub struct Panel {
    #[elem(child)]
    pub header: Label,
}

/// `Default`, so an element generic over its look has one too.
pub trait Look: Default + 'static {}

#[derive(Default)]
pub struct Dark;
#[derive(Default)]
pub struct Light;

impl Look for Dark {}
impl Look for Light {}

/// Generic, so its paths differ per argument.
#[element]
pub struct Themed<L: Look> {
    #[elem(child)]
    pub label: Label,
    #[elem(patch = Noop::<L>)]
    pub look: L,
}

fn show(what: &str, id: FieldId) {
    println!("  {what:<28} {id:?}");
}

fn main() {
    same_type_two_fields();
    same_name_two_owners();
    same_field_two_arguments();
    same_walk_twice();
    same_field_two_ways();
    hops_of_a_walk();
    whole_walk_keys();

    println!("\nall distinct.\n");
}

/// Two `Label` fields side by side. The type they hold is not what
/// names them.
fn same_type_two_fields() {
    println!("\ntwo fields of one type, in one struct");
    let top = Pair::cursor().top().hops()[0];
    let bottom = Pair::cursor().bottom().hops()[0];
    show("Pair::top", top);
    show("Pair::bottom", bottom);
    assert_ne!(top, bottom, "same type, but different paths");
}

/// A marker lives in its owner's module, so the owner is part of it.
fn same_name_two_owners() {
    println!("\nthe same field name in another struct");
    let pair = Pair::cursor().top().hops()[0];
    let stack = Stack::cursor().top().hops()[0];
    show("Pair::top", pair);
    show("Stack::top", stack);
    assert_ne!(pair, stack, "same name and type, other owner");
}

/// A generic struct has one marker per set of arguments.
fn same_field_two_arguments() {
    println!("\none generic struct, two arguments");
    let dark = Themed::<Dark>::cursor().label().hops()[0];
    let light = Themed::<Light>::cursor().label().hops()[0];
    show("Themed::<Dark>::label", dark);
    show("Themed::<Light>::label", light);
    assert_ne!(dark, light, "the marker carries the argument");
}

/// Nothing is hashed or allocated: an id is fixed at compile time.
fn same_walk_twice() {
    println!("\nthe same walk, twice");
    let once = Pair::cursor().top().hops()[0];
    let again = Pair::cursor().top().hops()[0];
    show("Pair::top", once);
    show("Pair::top", again);
    assert_eq!(once, again, "ids are stable");
}

/// Walking to a field and naming it from the enum are the same thing.
fn same_field_two_ways() {
    println!("\nthe same field, reached two ways");
    let walked = Pair::cursor().gap().hops()[0];
    let named = Pair::field_id(PairField::Gap);
    show("Pair::cursor().gap()", walked);
    show("PairField::Gap", named);
    assert_eq!(walked, named, "one field, one id");
}

/// One id per hop, so a walk says where it crosses into a child.
fn hops_of_a_walk() {
    println!("\ntwo hops name two ids");
    let ids = Pair::cursor().bottom().text().hops();
    show("Pair::bottom", ids[0]);
    show("  then Label::text", ids[1]);
    assert_eq!(ids[0], Pair::cursor().bottom().hops()[0]);

    // The second hop is `Label`'s own field, so `Label` recognises it
    // and `Pair` does not: a child patches itself.
    assert_eq!(Label::field(ids[1]), Some(LabelField::Text));
    assert!(Pair::field(ids[0]).is_none(), "`bottom` is an element");
}

/// A walk is one type however long, so it has one id: enough to key a
/// binding by the field it writes, however deep that field sits.
fn whole_walk_keys() {
    println!("\ntwo walks ending in the same hop");
    let top = Pair::cursor().top().text().key();
    let bottom = Pair::cursor().bottom().text().key();
    show("Pair::top().text()", top);
    show("Pair::bottom().text()", bottom);
    assert_ne!(top, bottom, "they differ at the first hop");

    println!("\nthe same walk under two owners");
    let card = Card::cursor().header().text().key();
    let panel = Panel::cursor().header().text().key();
    show("Card::header().text()", card);
    show("Panel::header().text()", panel);
    assert_ne!(card, panel, "the walks start somewhere different");

    println!("\ntwo fields of one child");
    let text = Pair::cursor().top().text().key();
    let size = Pair::cursor().top().size().key();
    show("Pair::top().text()", text);
    show("Pair::top().size()", size);
    assert_ne!(text, size, "they differ at the last hop");

    println!("\none walk, and a prefix of it");
    show("Pair::top()", Pair::cursor().top().key());
    assert_ne!(
        Pair::cursor().top().key(),
        top,
        "a shorter walk is a different type"
    );

    // A whole walk and a single hop are named in different ways, so
    // the two must not share a map.
    assert_ne!(
        Pair::cursor().top().key(),
        Pair::cursor().top().hops()[0],
        "a one hop walk is still a chain from the root"
    );
}

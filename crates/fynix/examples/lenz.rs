//! Walking to a field of a field of a field.
//!
//! Paths are not about UI: nothing here is an element, and there is no
//! backend. A walk is built from types, costs nothing at runtime, and
//! ends in one of three ways: reach the value, name the whole walk,
//! or list the hops it took.

use fynix::lenz::Lenz;

#[derive(Lenz)]
pub struct Card {
    pub header: Header,
}

#[derive(Lenz)]
pub struct Header {
    /// Optional, so a walk through it can come back empty.
    pub badge: Option<Badge>,
}

#[derive(Lenz)]
pub struct Badge {
    pub icon: Icon,
}

#[derive(Lenz)]
pub struct Icon {
    pub size: u32,
}

/// A card with every link present, its icon at 12.
fn card() -> Card {
    Card {
        header: Header {
            badge: Some(Badge {
                icon: Icon { size: 12 },
            }),
        },
    }
}

fn main() {
    reaching_the_value();
    writing_through_the_same_walk();
    absent_link_stops_the_walk();
    hops_are_the_route();
    the_key_is_the_whole_walk();

    println!();
}

/// Four hops across four structs, ending in a pair of function
/// pointers that read the field.
fn reaching_the_value() {
    let card = card();
    let size = Card::cursor().header().badge().icon().size();

    println!("\nreaching the value");
    println!("  size {:?}", size.accessor().get(&card));

    assert_eq!(size.accessor().get(&card), Some(&12));
}

/// The walk is `Copy` and zero sized, so the same one reads and
/// writes.
fn writing_through_the_same_walk() {
    let mut card = card();
    let size = Card::cursor().header().badge().icon().size();

    *size.accessor().get_mut(&mut card).unwrap() = 24;

    println!("\nwriting through the same walk");
    println!("  size {:?}", size.accessor().get(&card));

    assert_eq!(size.accessor().get(&card), Some(&24));
}

/// An `Option` field is just another hop. When it is empty the walk
/// answers `None` rather than panicking, and the caller never has to
/// know which link along the way was optional.
fn absent_link_stops_the_walk() {
    let card = Card {
        header: Header { badge: None },
    };
    let size = Card::cursor().header().badge().icon().size();

    println!("\nan absent link stops the walk");
    println!("  size {:?}", size.accessor().get(&card));

    assert_eq!(size.accessor().get(&card), None);
}

/// One id per hop, outermost first. This is the route a patch takes:
/// each owner recognises the first id and hands the rest on.
fn hops_are_the_route() {
    println!("\nhops are the route");

    let walks: [(&str, usize); 4] = [
        ("Card::header()", Card::cursor().header().hops().len()),
        ("  .badge()", Card::cursor().header().badge().hops().len()),
        (
            "  .icon()",
            Card::cursor().header().badge().icon().hops().len(),
        ),
        (
            "  .size()",
            Card::cursor()
                .header()
                .badge()
                .icon()
                .size()
                .hops()
                .len(),
        ),
    ];

    for (walk, hops) in walks {
        println!("  {walk:<18} {hops} hop(s)");
    }

    // The `Option` did not add a hop of its own: `badge` names one
    // field, whether or not it holds anything.
    assert_eq!(walks.map(|(_, hops)| hops), [1, 2, 3, 4]);

    // Each hop is a field of whoever owns it, named exactly as that
    // owner names it. That is what lets a patch be handed down: an
    // owner matches the first id against its own fields, then passes
    // the rest to whichever one answered.
    let hops = Card::cursor().header().badge().icon().size().hops();
    let owners = [
        ("Card::header", Card::cursor().header().hops()),
        ("Header::badge", Header::cursor().badge().hops()),
        ("Badge::icon", Badge::cursor().icon().hops()),
        ("Icon::size", Icon::cursor().size().hops()),
    ];

    println!(
        "\n  the whole route of .header().badge().icon().size()"
    );
    for (index, (owner, own)) in owners.iter().enumerate() {
        println!("    {index} {owner:<14} {:?}", hops[index]);
        assert_eq!(
            hops[index], own[0],
            "each hop is the owner's own"
        );
    }
}

/// However many hops, a walk is one type, so one id names all of it.
fn the_key_is_the_whole_walk() {
    println!("\nthe key is the whole walk");

    let size = Card::cursor().header().badge().icon().size().key();
    let icon = Card::cursor().header().badge().icon().key();
    println!("  four hops {size:?}");
    println!("  three     {icon:?}");

    assert_ne!(size, icon, "a prefix is a different walk");
    assert_eq!(
        size,
        Card::cursor().header().badge().icon().size().key(),
        "and the same walk is always the same key"
    );
}

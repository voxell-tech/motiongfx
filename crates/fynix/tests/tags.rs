//! Tags drive a field to whichever source its highest active line
//! names, and back down the list as tags come off.

mod common;

use core::time::Duration;

use common::{FynixHost, World};
use fynix::element::element;
use fynix::host::Host;
use fynix::{Fynix, WorldNodeRef, elem};
use motiongfx_interp::ease;

/// Tags are plain types. Nothing here implements a fynix trait.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hovered;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pressed;

#[element(host = FynixHost)]
pub struct Button {
    #[elem(default = 0, patch = WriteSize, anim(
        ms = 100,
        ease = ease::linear,
        on(Pressed, read = press_size),
        on(Hovered, read = hover_size),
    ))]
    pub size: u32,

    /// Answers to `Hovered` only, which is what a press must not
    /// disturb.
    #[elem(default = 0, patch = WriteBorder, anim(
        ms = 100,
        ease = ease::linear,
        on(Hovered, read = hover_border),
    ))]
    pub border: u32,

    /// Element state the lines read through; nothing draws them.
    pub hover_size: u32,
    pub press_size: u32,
    pub hover_border: u32,
}

test_patch!(WriteSize, u32, |patch, v| {
    let node = patch.id();
    patch.world.node(node).size = *v;
});

test_patch!(WriteBorder, u32, |patch, v| {
    let node = patch.id();
    patch.world.node(node).border_width = *v;
});

/// Fires on the first flush and never again, so the node a test
/// captured stays put.
fn once() -> impl for<'w> FnMut(WorldNodeRef<'w, FynixHost>) -> bool
+ Send
+ Sync
+ 'static {
    let mut pending = true;
    move |_| core::mem::take(&mut pending)
}

/// A button whose `size` runs base -> hover -> press and whose
/// `border` only ever answers to `Hovered`.
///
/// One flush is a quarter of a leg, so a leg takes four.
fn button() -> (World, Fynix<FynixHost>, usize) {
    let (mut world, root) = World::with_root();
    world.delta = Duration::from_millis(25);

    let mut kernel = Fynix::new(());
    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(
                Button,
                size = 0u32,
                border = 0u32,
                hover_size = 100u32,
                press_size = 200u32,
                hover_border = 40u32,
            ));
        },
        &mut world,
    );

    let node = FynixHost::children(&world, root)[0];
    (world, kernel, node)
}

/// Run `n` flushes.
fn flush(kernel: &mut Fynix<FynixHost>, world: &mut World, n: usize) {
    for _ in 0..n {
        kernel.flush(world);
    }
}

#[test]
fn tag_travels_to_its_line() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(
        world.get(node).size,
        100,
        "arrived at the hover size"
    );
    assert_eq!(world.get(node).border_width, 40, "border too");
}

#[test]
fn higher_line_wins_while_both_are_set() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 4);
    kernel.set_tag(node, Pressed);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(
        world.get(node).size,
        200,
        "`Pressed` is listed above `Hovered`"
    );
}

/// The case the priority model exists for: a field with no `Pressed`
/// line must not fall to its base while the pointer is still over it.
#[test]
fn line_field_lacks_leaves_it_alone() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 4);
    kernel.set_tag(node, Pressed);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(
        world.get(node).border_width,
        40,
        "`border` never mentions `Pressed`, so it holds its hover"
    );
}

/// Releasing drops one tag, and the field falls back to the next
/// line still active - not to base.
#[test]
fn dropping_tag_falls_back_to_next_line() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 4);
    kernel.set_tag(node, Pressed);
    flush(&mut kernel, &mut world, 4);

    kernel.unset_tag::<Pressed>(node);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(
        world.get(node).size,
        100,
        "back to hover, which was never unset"
    );
}

#[test]
fn dropping_every_tag_returns_to_base() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 4);
    kernel.unset_tag::<Hovered>(node);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(world.get(node).size, 0);
    assert_eq!(world.get(node).border_width, 0);
}

#[test]
fn settled_field_holds_no_row() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    assert!(kernel.moving_len() > 0, "a leg is running");

    flush(&mut kernel, &mut world, 4);
    assert_eq!(kernel.moving_len(), 0, "arrival drops the row");
}

/// Interrupting mid-leg carries on from where it had reached, and a
/// reverse only takes back the time it had spent.
#[test]
fn reversing_cuts_the_duration() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 1);
    assert_eq!(world.get(node).size, 25, "a quarter of the way");

    kernel.unset_tag::<Hovered>(node);
    flush(&mut kernel, &mut world, 1);

    assert_eq!(
        world.get(node).size,
        0,
        "one flush undoes the one flush it had spent"
    );
}

/// A per-line `ms` overrides the `anim` default.
#[test]
fn line_can_set_its_own_duration() {
    #[element(host = FynixHost)]
    pub struct Slow {
        #[elem(default = 0, patch = WriteSize, anim(
            ms = 100,
            ease = ease::linear,
            on(Hovered, read = hover_size, ms = 200),
        ))]
        pub size: u32,
        #[elem(ignore)]
        pub hover_size: u32,
    }

    let (mut world, root) = World::with_root();
    world.delta = Duration::from_millis(50);
    let mut kernel = Fynix::new(());
    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(Slow, size = 0u32, hover_size = 100u32));
        },
        &mut world,
    );
    let node = FynixHost::children(&world, root)[0];

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 2);

    assert_eq!(
        world.get(node).size,
        50,
        "halfway after 100ms of a 200ms line"
    );
}

/// A node that dies mid-leg takes its row with it, and a resting
/// tree holds none at all.
#[test]
fn despawning_drops_the_row() {
    let (mut world, mut kernel, node) = button();

    kernel.set_tag(node, Hovered);
    flush(&mut kernel, &mut world, 1);
    assert_eq!(kernel.moving_len(), 2, "both fields are travelling");

    FynixHost::despawn(&mut world, node);
    kernel.flush(&mut world);

    assert_eq!(kernel.moving_len(), 0, "the sweep took them");
}

/// A `#[elem(child)]` two levels deep resolves through both of its
/// mount points, not just one hop up.
#[test]
fn a_grandchild_still_resolves_its_own_field() {
    #[element(host = FynixHost)]
    pub struct Leaf {
        #[elem(default = 0, patch = WriteSize, anim(
            ms = 100,
            ease = ease::linear,
            on(Hovered, read = hover_size),
        ))]
        pub size: u32,
        #[elem(ignore)]
        pub hover_size: u32,
    }

    #[element(host = FynixHost)]
    pub struct Middle {
        #[elem(child)]
        pub leaf: Leaf,
    }

    #[element(host = FynixHost)]
    pub struct Outer {
        #[elem(child)]
        pub middle: Middle,
    }

    let (mut world, root) = World::with_root();
    world.delta = Duration::from_millis(25);
    let mut kernel = Fynix::new(());
    kernel.watch(
        root,
        once(),
        |ui| {
            ui.elem(elem!(
                Outer,
                middle = elem!(
                    Middle,
                    leaf =
                        elem!(Leaf, size = 0u32, hover_size = 100u32)
                )
            ));
        },
        &mut world,
    );

    let outer = FynixHost::children(&world, root)[0];
    let middle = FynixHost::children(&world, outer)[0];
    let leaf = FynixHost::children(&world, middle)[0];

    kernel.set_tag(leaf, Hovered);
    flush(&mut kernel, &mut world, 4);

    assert_eq!(
        world.get(leaf).size,
        100,
        "arrived through both hops"
    );
}

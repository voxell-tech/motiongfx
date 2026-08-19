//! What a lane does: travel to what it was aimed at, carry on from
//! wherever it is when that changes, and give the element back when it
//! lets go.

mod common;

use common::{Backend, Interact, Label, LabelCursor, TestAim, World};
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::host::Host;
use fynix_mock::lenz::Lenz;
use fynix_mock::style::Style;
use fynix_mock::transition::Transition;
use fynix_mock::ui::Ui;
use fynix_mock::{Fynix, OverrideDefault, elem};
use motiongfx_interp::ease;
use motiongfx_interp::interpolation::Interpolation;

/// Fires on the first flush and never again.
fn once() -> impl FnMut(&World, usize) -> bool + Send + Sync + 'static
{
    let mut pending = true;
    move |_, _| core::mem::take(&mut pending)
}

fn only_child(world: &World, root: usize) -> usize {
    let children = Backend::children(world, root);
    assert_eq!(children.len(), 1, "one element was built");
    children[0]
}

/// A label whose size travels over a second, and the world it is in.
/// The delta is a quarter of that, so a flush is a quarter of the way.
fn travelling() -> (World, usize, Fynix<Backend>, usize) {
    let (mut world, root) = World::with_root();
    world.delta = 0.25;
    let mut kernel = Fynix::new();

    kernel.watch(root, once(), |ui| {
        ui.elem(elem!(Label, size = 0u32)).transition(
            |label| label.size(),
            Transition::secs(1.0, <u32 as Interpolation<()>>::interp)
                .ease(ease::linear),
        );
    });
    kernel.flush(&mut world);

    let label = only_child(&world, root);
    (world, root, kernel, label)
}

#[test]
fn base_is_what_shows_until_something_aims_the_lane() {
    let (mut world, _, mut kernel, label) = travelling();

    assert_eq!(world.get(label).size, 0);

    kernel.flush(&mut world);
    kernel.flush(&mut world);

    assert_eq!(world.get(label).size, 0, "nothing aimed it");
}

#[test]
fn aimed_field_travels_over_the_transition() {
    let (mut world, _, mut kernel, label) = travelling();

    kernel.aim::<Label, _>(label, |label| label.size(), Some(100));

    for expected in [25, 50, 75, 100] {
        kernel.flush(&mut world);
        assert_eq!(world.get(label).size, expected);
    }

    kernel.flush(&mut world);
    assert_eq!(world.get(label).size, 100, "and stays arrived");
}

#[test]
fn retarget_carries_on_from_where_it_reached() {
    let (mut world, _, mut kernel, label) = travelling();

    kernel.aim::<Label, _>(label, |label| label.size(), Some(100));
    kernel.flush(&mut world);
    kernel.flush(&mut world);
    assert_eq!(world.get(label).size, 50, "halfway to the first");

    // A second target mid flight starts its own leg from here, rather
    // than from where the first one started.
    kernel.aim::<Label, _>(label, |label| label.size(), Some(150));
    kernel.flush(&mut world);

    assert_eq!(world.get(label).size, 75);
}

#[test]
fn releasing_travels_back_to_the_base() {
    let (mut world, _, mut kernel, label) = travelling();

    kernel.aim::<Label, _>(label, |label| label.size(), Some(100));
    for _ in 0..4 {
        kernel.flush(&mut world);
    }
    assert_eq!(world.get(label).size, 100);

    kernel.aim::<Label, _>(label, |label| label.size(), None);

    for expected in [75, 50, 25, 0] {
        kernel.flush(&mut world);
        assert_eq!(world.get(label).size, expected);
    }
}

#[test]
fn element_keeps_the_base_while_a_lane_is_in_flight() {
    let (mut world, root, mut kernel, label) = travelling();

    kernel.aim::<Label, _>(label, |label| label.size(), Some(100));
    kernel.flush(&mut world);
    assert_eq!(world.get(label).size, 25, "the backend moved");

    // Whatever the lane shows, the element is still what the cascade
    // left: a rebuild starts from the base, not from mid flight.
    kernel.unwatch(root);
    kernel.watch(root, once(), |ui| {
        ui.elem(elem!(Label, size = 0u32));
    });
    kernel.flush(&mut world);

    let rebuilt = only_child(&world, root);
    assert_eq!(world.get(rebuilt).size, 0);
}

#[test]
fn style_carries_what_moves_as_well_as_what_it_looks_like() {
    /// A style has no node to wire a lane onto, so what moves is the
    /// element's own business - `Grower` leaves a slot for a style to
    /// fill, and wires the lane itself once it has a node to put it
    /// on.
    #[derive(Element, OverrideDefault, Lenz)]
    pub struct Grower {
        #[default(String::from("Label"))]
        pub text: String,
        #[default(13)]
        pub size: u32,
        /// What a style asks this to grow to under the pointer, if
        /// anything.
        pub grows_to: Option<u32>,
    }

    impl ElementVisual<Backend> for Grower {
        fn build_fields(&self, ui: &mut Ui<'_, Backend>) {
            let node = ui.parent();
            ui.world.node(node).text = self.text.clone();
            ui.world.node(node).size = self.size;

            if let Some(target) = self.grows_to {
                ui.this::<Grower>()
                    .transition_from(
                        |g| g.size(),
                        self.size,
                        Transition::secs(
                            1.0,
                            <u32 as Interpolation<()>>::interp,
                        ),
                    )
                    .aim_on(
                        Interact::Enter,
                        |g| g.size(),
                        Some(target),
                    )
                    .aim_on(Interact::Leave, |g| g.size(), None);
            }
        }

        fn patch_fields(
            &self,
            world: &mut World,
            node: usize,
            field: GrowerField,
        ) {
            match field {
                GrowerField::Text => {
                    world.node(node).text = self.text.clone()
                }
                GrowerField::Size => {
                    world.node(node).size = self.size
                }
                // Read once, at build - see build_fields.
                GrowerField::GrowsTo => {}
            }
        }
    }

    /// Both halves of a look: a size, and the size it goes to under
    /// the pointer.
    struct Grows;

    impl Style for Grows {
        type Host = Backend;
        type Element = Grower;

        fn apply(self, grower: &mut Grower) {
            grower.size = 10;
            grower.grows_to = Some(20);
        }
    }

    let (mut world, root) = World::with_root();
    world.delta = 0.5;
    let mut kernel = Fynix::new();

    kernel.watch(root, once(), |ui| {
        ui.elem(elem!(!Grows, text = "Save"));
    });
    kernel.flush(&mut world);

    let label = only_child(&world, root);
    assert_eq!(world.get(label).size, 10, "the style's own");
    assert_eq!(world.get(label).text, "Save", "and the call site's");

    world.interact(&mut kernel, label, Interact::Enter);
    kernel.flush(&mut world);
    assert_eq!(world.get(label).size, 15, "halfway up");

    world.interact(&mut kernel, label, Interact::Leave);
    kernel.flush(&mut world);
    assert_eq!(world.get(label).size, 12, "and back down from there");
}

#[test]
fn lane_goes_with_the_node_it_was_declared_on() {
    let (mut world, _, mut kernel, label) = travelling();

    assert_eq!(kernel.lane_len(), 1);

    Backend::despawn(&mut world, label);
    kernel.flush(&mut world);

    assert_eq!(kernel.lane_len(), 0);
}

//! One field's write to the backend, and the traits that name one.

use crate::host::Host;
use crate::lenz::{FieldPath, Tagged};

/// What a `#[elem(patch = ...)]` writer writes through: the node the
/// value lands on, `world`, and `theme`.
///
/// No [`Store`](crate::store::Store) or
/// [`AnimTable`](crate::anim::AnimTable) here. A patch writes a
/// value; it wires nothing.
pub struct Patch<'a, H: Host> {
    pub world: &'a mut H::World,
    pub theme: &'a H::Theme,
    node: H::Node,
}

impl<'a, H: Host> Patch<'a, H> {
    /// Not for hand-written code.
    #[doc(hidden)]
    pub fn new(
        world: &'a mut H::World,
        node: H::Node,
        theme: &'a H::Theme,
    ) -> Self {
        Self { world, theme, node }
    }

    /// This element's own node.
    pub fn id(&self) -> H::Node {
        self.node
    }
}

/// One field's writer, implemented on the tag a `#[elem(patch = ...)]`
/// field carries.
pub trait FieldPatch<H: Host> {
    type Target;

    fn patch(patch: &mut Patch<H>, value: &Self::Target);
}

/// A field path whose terminal hop is a `#[elem(patch = ...)]` field.
pub trait Bindable<H: Host>: FieldPath {
    fn patch(patch: &mut Patch<H>, value: &Self::Target);
}

impl<P, H> Bindable<H> for P
where
    H: Host,
    P: FieldPath + Tagged,
    P::Tag: FieldPatch<H, Target = P::Target>,
{
    fn patch(patch: &mut Patch<H>, value: &P::Target) {
        <P::Tag as FieldPatch<H>>::patch(patch, value)
    }
}

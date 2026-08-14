//! Building a subtree from inputs, without becoming part of the tree.
//!
//! An [`Element`] is data the kernel keeps: it is stored against the
//! node it built, and a change to one of its fields is *patched* onto
//! that node. A composer is none of that. It is consumed the moment
//! [`Ui::compose`] runs it, nothing about it survives the build, and
//! so it may hold borrows an element never could.
//!
//! The split is about what an input decides. A field like a colour
//! decides how a node *looks*, and patching it is what keeps the node
//! alive across the change. An input like "which entity" decides what
//! the subtree *is*, and there is no patch that means "build something
//! else instead" — so it is read once, while building, which is
//! exactly the window a composer has.
//!
//! Unlike its counterpart in `fynix`, this carries no `Style` of its
//! own: there is no ambient style chain here for one to be built from,
//! so a composer's own fields are already the whole of its input.

use crate::element::Element;
use crate::host::Host;
use crate::ui::{ElementHandle, Ui};

/// Builds a subtree from whatever it was handed.
pub trait Composer<H: Host> {
    /// What the subtree hangs from. It is what the caller gets back,
    /// so this is the element a binding made against the result will
    /// be checked against.
    type Element: Element<H>;

    /// Build it, and name the root.
    ///
    /// The handle is returned rather than the [`ElementMut`] itself
    /// because that one still borrows `ui`, and this has to give the
    /// borrow up to return at all.
    ///
    /// [`ElementMut`]: crate::ui::ElementMut
    fn compose(
        self,
        ui: &mut Ui<'_, H>,
    ) -> ElementHandle<H, Self::Element>;
}

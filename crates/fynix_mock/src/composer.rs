//! Building a subtree from inputs, without becoming part of the tree.
//!
//! An [`Element`] is stored against the node it built, and a change to
//! one of its fields is patched onto that node. A composer is neither:
//! [`Ui::compose`] consumes it, so it may hold borrows an element
//! never could.
//!
//! Which to reach for is decided by what the input means. A colour
//! decides how a node looks, and patching it is what keeps the node
//! alive across the change. "Which entity" decides what the subtree
//! *is*, and there is no patch for that - so it is read once, while
//! building.
//!
//! Unlike `fynix`'s, this carries no `Style`: there is no ambient
//! style chain here to build one from.

use crate::element::Element;
use crate::host::Host;
use crate::ui::{ElementHandle, Ui};

/// Builds a subtree from whatever it was handed.
pub trait Composer<H: Host> {
    /// What the subtree hangs from, and what a binding made against
    /// the result is checked against.
    type Element: Element<H>;

    /// Build it, and name the root.
    ///
    /// A handle rather than the [`ElementMut`], which still borrows
    /// `ui`.
    ///
    /// [`ElementMut`]: crate::ui::ElementMut
    fn compose(
        self,
        ui: &mut Ui<'_, H>,
    ) -> ElementHandle<H, Self::Element>;
}

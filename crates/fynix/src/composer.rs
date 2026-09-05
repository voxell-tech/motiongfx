//! Building a subtree from inputs, without becoming part of the tree.
//!
//! An [`Element`] is stored against the node it built, so a changed
//! field can be patched. A composer is consumed by [`Ui::compose`]
//! instead, so it can hold borrows an element never could.
//!
//! Use a composer when the input decides what the subtree *is*, not
//! how it looks. There's no patch for that: it's read once, at
//! build.

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

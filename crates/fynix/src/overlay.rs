//! A value laid over a field while it transitions, kept beside the
//! element rather than in it.
//!
//! The element carries the *base*, the cascade's own value. An overlay
//! carries what the backend is showing and writes it every frame,
//! straight through the field's `#[elem(patch = ...)]` tag.

use alloc::boxed::Box;
use core::any::Any;

use hashbrown::HashMap;

use crate::host::Host;
use crate::lenz::FieldId;
use crate::transition::Transition;
use crate::ui::Patch;

/// A field with a transitioning value over it, ticked once a frame.
///
/// The erased view of a [`Tween`]: the flush loop advances every
/// overlay without knowing the value type.
pub(crate) trait Overlay<H: Host>: Any + Send + Sync {
    /// Advance by `dt` and write what it reached. `false` once it has
    /// arrived home and the base is what shows.
    fn advance(
        &mut self,
        dt: f32,
        world: &mut H::World,
        node: H::Node,
        theme: &H::Theme,
    ) -> bool;
}

/// One field's transition in progress.
pub(crate) struct Tween<H: Host, T> {
    /// The field's `#[elem(patch = ...)]` writer.
    write: fn(&mut Patch<H>, &T),
    curve: Transition<T>,
    /// The cascade's own value. A binding on the same field moves it
    /// through [`rebase`](Self::rebase).
    base: T,
    /// Where it is aimed, or `None` while heading home to the base.
    target: Option<T>,
    /// The current leg's start.
    from: T,
    elapsed: f32,
}

impl<H: Host, T: Clone> Tween<H, T> {
    fn heading(&self) -> &T {
        self.target.as_ref().unwrap_or(&self.base)
    }

    fn shown(&self) -> T {
        if self.curve.done(self.elapsed) {
            self.heading().clone()
        } else {
            (self.curve.lerp)(
                &self.from,
                self.heading(),
                self.curve.at(self.elapsed),
            )
        }
    }

    /// Aim at `target`, or `None` to release it back to the base.
    pub(crate) fn aim(&mut self, target: Option<T>) {
        // Snapshot before the endpoint moves.
        self.from = self.shown();
        self.elapsed = 0.0;
        self.target = target;
    }

    /// The cascade value moved. Start a fresh leg if that is where it
    /// is heading.
    pub(crate) fn rebase(&mut self, base: &T) {
        if self.target.is_none() {
            self.from = self.shown();
            self.elapsed = 0.0;
        }
        self.base = base.clone();
    }
}

impl<H, T> Overlay<H> for Tween<H, T>
where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    fn advance(
        &mut self,
        dt: f32,
        world: &mut H::World,
        node: H::Node,
        theme: &H::Theme,
    ) -> bool {
        // Arrived home: nothing to write, the base shows.
        if self.target.is_none() && self.curve.done(self.elapsed) {
            return false;
        }

        self.elapsed += dt;
        let shown = self.shown();

        // Written every frame, even unmoved, so a binding that wrote
        // the base earlier this flush cannot win.
        let mut patch = Patch::new(world, node, theme);
        (self.write)(&mut patch, &shown);

        true
    }
}

/// Every field with an overlay. One per field - a second overlay on
/// the same field replaces rather than doubles.
pub struct Overlays<H: Host>(
    HashMap<(H::Node, FieldId), Box<dyn Overlay<H>>>,
);

impl<H: Host> Default for Overlays<H> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<H: Host> Overlays<H> {
    pub(crate) fn insert(
        &mut self,
        node: H::Node,
        key: FieldId,
        overlay: Box<dyn Overlay<H>>,
    ) {
        self.0.insert((node, key), overlay);
    }

    /// The overlay on `(node, key)`, downcast to the `Tween<H, T>` it
    /// must be. `None` if there is no overlay there.
    pub(crate) fn tween<T: 'static>(
        &mut self,
        node: H::Node,
        key: FieldId,
    ) -> Option<&mut Tween<H, T>> {
        let overlay = self.0.get_mut(&(node, key))?.as_mut();
        let any: &mut dyn Any = overlay;
        let tween = any.downcast_mut();
        debug_assert!(
            tween.is_some(),
            "an overlay's value type does not match the field's"
        );
        tween
    }

    pub(crate) fn retain(
        &mut self,
        mut keep: impl FnMut(H::Node) -> bool,
    ) {
        self.0.retain(|(node, _), _| keep(*node));
    }

    pub(crate) fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (H::Node, &mut Box<dyn Overlay<H>>)>
    {
        self.0
            .iter_mut()
            .map(|((node, _), overlay)| (*node, overlay))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Lay an overlay over `key` on `node`, starting from `base`.
pub(crate) fn insert_overlay<H, T>(
    overlays: &mut Overlays<H>,
    node: H::Node,
    key: FieldId,
    write: fn(&mut Patch<H>, &T),
    base: T,
    curve: Transition<T>,
) where
    H: Host,
    T: Clone + Send + Sync + 'static,
{
    overlays.insert(
        node,
        key,
        Box::new(Tween {
            write,
            from: base.clone(),
            base,
            target: None,
            // Settled on the base until something aims it.
            elapsed: curve.duration,
            curve,
        }),
    );
}

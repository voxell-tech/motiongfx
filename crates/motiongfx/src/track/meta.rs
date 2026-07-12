//! The composition tree recording how [`TrackFragment`]s were combined
//! (chain / all / any / flow) to build a [`Track`].
//!
//! Only compiled with the `metadata` feature. Used to visualize the
//! command structure (e.g. in the editor), not for playback.
//!
//! [`TrackFragment`]: super::TrackFragment
//! [`Track`]: super::Track

use alloc::vec::Vec;

use crate::action::{ActionClip, ActionId, ActionKey};

/// How a group of child fragments were combined in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    /// Sequential, one after another ([`ord_chain`](super::TrackOrdering::ord_chain)).
    Chain,
    /// Concurrent, waiting for the longest ([`ord_all`](super::TrackOrdering::ord_all)).
    All,
    /// Concurrent, waiting for the shortest ([`ord_any`](super::TrackOrdering::ord_any)).
    Any,
    /// Sequential, each offset from the previous by a fixed delay
    /// ([`ord_flow`](super::TrackOrdering::ord_flow)).
    Flow(f32),
}

/// A node in a [`Track`](super::Track)'s composition tree.
#[derive(Debug, Clone, PartialEq)]
pub struct FragmentMeta {
    /// Start time of this node, relative to the track start.
    pub start: f32,
    /// Duration of this node.
    pub duration: f32,
    /// Whether this is a single action or a combination.
    pub kind: FragmentKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FragmentKind {
    /// A single action (one `.play()`).
    Leaf {
        /// Identifies the target subject and field (used for labels).
        key: ActionKey,
        /// The action entity, to correlate a node with its action.
        id: ActionId,
    },
    /// A combination of child fragments.
    Group {
        combinator: Combinator,
        children: Vec<FragmentMeta>,
    },
}

impl FragmentMeta {
    /// A leaf node for a single clip, starting at 0.
    pub(super) fn leaf(key: ActionKey, clip: &ActionClip) -> Self {
        Self {
            start: 0.0,
            duration: clip.duration,
            kind: FragmentKind::Leaf { key, id: clip.id },
        }
    }

    /// A group node starting at 0.
    pub(super) fn group(
        combinator: Combinator,
        duration: f32,
        children: Vec<FragmentMeta>,
    ) -> Self {
        Self {
            start: 0.0,
            duration,
            kind: FragmentKind::Group {
                combinator,
                children,
            },
        }
    }

    /// The neutral node for an empty fragment.
    pub(super) fn empty() -> Self {
        Self::group(Combinator::Chain, 0.0, Vec::new())
    }

    /// Offset this node (and its whole subtree) later in time.
    pub(super) fn shift(&mut self, dt: f32) {
        self.start += dt;
        if let FragmentKind::Group { children, .. } = &mut self.kind {
            for child in children.iter_mut() {
                child.shift(dt);
            }
        }
    }
}

impl Default for FragmentMeta {
    fn default() -> Self {
        Self::empty()
    }
}

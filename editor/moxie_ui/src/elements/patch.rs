//! Shared `#[elem(patch = ...)]` writers.
//!
//! A field writer only sees its own value, so the pattern is: an
//! element's `#[element(build = ...)]` hook inserts the component
//! whole, and a writer here reaches that component back and moves the
//! one field. These are the writers many elements share; the ones
//! specific to a single element live beside it.

use bevy::ecs::component::Mutable;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut as _;
use fynix::ui::Patch;

use crate::reactive::FynixHost;

/// Move one field of component `C` on the node a patch landed on, if
/// the node still carries one.
pub(super) fn with<C: Component<Mutability = Mutable>>(
    patch: &mut Patch<FynixHost>,
    f: impl FnOnce(&mut C),
) {
    if let Some(mut component) = patch.entity_mut().get_mut::<C>() {
        f(&mut component);
    }
}

/// As [`with`], for [`Node`] - the component almost every writer
/// touches.
pub(super) fn node(
    patch: &mut Patch<FynixHost>,
    f: impl FnOnce(&mut Node),
) {
    with(patch, f);
}

/// One `Node` field, set from a reference to a matching value.
macro_rules! set {
    ($name:ident, $ty:ty, |$n:ident, $v:ident| $body:expr) => {
        pub(super) fn $name(patch: &mut Patch<FynixHost>, $v: &$ty) {
            node(patch, |$n| $body);
        }
    };
}

set!(width, Val, |n, v| n.width = *v);
set!(height, Val, |n, v| n.height = *v);
set!(min_width, Val, |n, v| n.min_width = *v);
set!(min_height, Val, |n, v| n.min_height = *v);
set!(max_width, Val, |n, v| n.max_width = *v);
set!(row_gap, Val, |n, v| n.row_gap = *v);
set!(column_gap, Val, |n, v| n.column_gap = *v);
set!(radius, Val, |n, v| n.border_radius = BorderRadius::all(*v));
set!(flex_grow, f32, |n, v| n.flex_grow = *v);
set!(flex_shrink, f32, |n, v| n.flex_shrink = *v);
set!(padding, UiRect, |n, v| n.padding = *v);
set!(margin, UiRect, |n, v| n.margin = *v);
set!(direction, FlexDirection, |n, v| n.flex_direction = *v);
set!(align, AlignItems, |n, v| n.align_items = *v);
set!(justify, JustifyContent, |n, v| n.justify_content = *v);
set!(position, PositionType, |n, v| n.position_type = *v);
set!(display, Display, |n, v| n.display = *v);
set!(overflow, Overflow, |n, v| n.overflow = *v);
set!(top, Val, |n, v| n.top = *v);
set!(left, Val, |n, v| n.left = *v);

/// The four edge insets, from one [`UiRect`].
pub(super) fn inset(patch: &mut Patch<FynixHost>, v: &UiRect) {
    node(patch, |n| {
        n.left = v.left;
        n.right = v.right;
        n.top = v.top;
        n.bottom = v.bottom;
    });
}

pub(super) fn background(patch: &mut Patch<FynixHost>, v: &Color) {
    patch.insert(BackgroundColor(*v));
}

pub(super) fn border_color(patch: &mut Patch<FynixHost>, v: &Color) {
    patch.insert(BorderColor::all(*v));
}

pub(super) fn z_index(patch: &mut Patch<FynixHost>, v: &i32) {
    patch.insert(GlobalZIndex(*v));
}

/// `None` drops the override, leaving the node in its parent's stack.
pub(super) fn optional_z(
    patch: &mut Patch<FynixHost>,
    v: &Option<i32>,
) {
    match v {
        Some(z) => {
            patch.insert(GlobalZIndex(*z));
        }
        None => {
            patch.remove::<GlobalZIndex>();
        }
    }
}

// ---------------------------------------------------------------------
// shared, but shaped by one element's layout
// ---------------------------------------------------------------------

/// A square icon: one value drives both dimensions.
pub(super) fn icon_size(patch: &mut Patch<FynixHost>, size: &Val) {
    node(patch, |n| {
        n.width = *size;
        n.height = *size;
    });
}

/// An overlay never blocks; what it catches, it catches by being seen.
pub(super) fn overlay_pickable(
    patch: &mut Patch<FynixHost>,
    catches: &bool,
) {
    patch.insert(Pickable {
        should_block_lower: false,
        is_hoverable: *catches,
    });
}

pub(super) fn panel_scroll(
    patch: &mut Patch<FynixHost>,
    scroll: &f32,
) {
    with::<ScrollPosition>(patch, |pos| pos.0.y = *scroll);
}

pub(super) fn panel_scrolls(
    patch: &mut Patch<FynixHost>,
    scrolls: &bool,
) {
    let overflow = if *scrolls {
        Overflow::scroll_y()
    } else {
        Overflow::clip()
    };
    node(patch, move |n| n.overflow = overflow);
}

fn axis(on: bool) -> OverflowAxis {
    if on {
        OverflowAxis::Scroll
    } else {
        OverflowAxis::Visible
    }
}

pub(super) fn scroll_x(patch: &mut Patch<FynixHost>, on: &bool) {
    let a = axis(*on);
    node(patch, move |n| n.overflow.x = a);
}

pub(super) fn scroll_y(patch: &mut Patch<FynixHost>, on: &bool) {
    let a = axis(*on);
    node(patch, move |n| n.overflow.y = a);
}

/// A time-axis label's `left`, plus the centring its position implies:
/// the leftmost mark reads flush, the rest centre over their tick.
pub(super) fn time_label_x(patch: &mut Patch<FynixHost>, x: &Val) {
    let x = *x;
    let centred = !matches!(x, Val::Px(p) if p <= 0.0);
    node(patch, move |n| {
        n.left = x;
        n.justify_content = if centred {
            JustifyContent::Center
        } else {
            JustifyContent::FlexStart
        };
        n.padding =
            UiRect::left(if centred { Val::ZERO } else { px(3) });
    });
}

/// A timeline track, sized to its duration on both `width` and its
/// own min.
pub(super) fn track_width(patch: &mut Patch<FynixHost>, width: &Val) {
    let width = *width;
    node(patch, move |n| {
        n.width = width;
        n.min_width = width;
    });
}

/// A timeline clip's border thickens when it is selected.
pub(super) fn selected(
    patch: &mut Patch<FynixHost>,
    selected: &bool,
) {
    let w = if *selected { 2 } else { 1 };
    node(patch, move |n| n.border = UiRect::all(px(w)));
}

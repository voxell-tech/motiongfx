//! Shared `#[elem(patch = ...)]` tags.
//!
//! A field writer only sees its own value, so the pattern is: an
//! element's `#[element(build = ...)]` hook inserts the component
//! whole, and the tag here reaches that component back and moves the
//! one field. These are the tags many elements share; the ones
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

/// As [`with`], for [`Node`] - the component almost every tag touches.
pub(super) fn node(
    patch: &mut Patch<FynixHost>,
    f: impl FnOnce(&mut Node),
) {
    with(patch, f);
}

/// Defines a `#[elem(patch = ...)]` tag: a unit struct that writes one
/// field value to the backend. Always `pub` - a lenz marker is `pub`
/// and its `type Tag` cannot be less visible.
macro_rules! field_patch {
    (
        $(#[$meta:meta])*
        $name:ident, $ty:ty,
        |$p:ident, $v:ident| $body:expr
    ) => {
        $(#[$meta])*
        pub struct $name;

        impl ::fynix::ui::FieldPatch<$crate::reactive::FynixHost>
            for $name
        {
            type Target = $ty;

            fn patch(
                $p: &mut ::fynix::ui::Patch<
                    $crate::reactive::FynixHost,
                >,
                $v: &$ty,
            ) {
                $body
            }
        }
    };
}
pub(crate) use field_patch;

/// A [`field_patch!`] tag that only moves one [`Node`] field.
macro_rules! set_node {
    (
        $(#[$meta:meta])*
        $name:ident, $ty:ty,
        |$n:ident, $v:ident| $body:expr
    ) => {
        $crate::elements::patch::field_patch!(
            $(#[$meta])*
            $name, $ty,
            |__patch, $v| $crate::elements::patch::node(__patch, |$n| $body)
        );
    };
}

set_node!(PatchWidth, Val, |n, v| n.width = *v);
set_node!(PatchHeight, Val, |n, v| n.height = *v);
set_node!(PatchMinWidth, Val, |n, v| n.min_width = *v);
set_node!(PatchMinHeight, Val, |n, v| n.min_height = *v);
set_node!(PatchMaxWidth, Val, |n, v| n.max_width = *v);
set_node!(PatchRowGap, Val, |n, v| n.row_gap = *v);
set_node!(PatchColumnGap, Val, |n, v| n.column_gap = *v);
set_node!(PatchRadius, Val, |n, v| n.border_radius =
    BorderRadius::all(*v));
set_node!(PatchFlexGrow, f32, |n, v| n.flex_grow = *v);
set_node!(PatchFlexShrink, f32, |n, v| n.flex_shrink = *v);
set_node!(PatchPadding, UiRect, |n, v| n.padding = *v);
set_node!(PatchMargin, UiRect, |n, v| n.margin = *v);
set_node!(PatchDirection, FlexDirection, |n, v| n.flex_direction =
    *v);
set_node!(PatchAlign, AlignItems, |n, v| n.align_items = *v);
set_node!(PatchJustify, JustifyContent, |n, v| n.justify_content =
    *v);
set_node!(PatchPosition, PositionType, |n, v| n.position_type = *v);
set_node!(PatchDisplay, Display, |n, v| n.display = *v);
set_node!(PatchOverflow, Overflow, |n, v| n.overflow = *v);
set_node!(PatchTop, Val, |n, v| n.top = *v);
set_node!(PatchLeft, Val, |n, v| n.left = *v);

field_patch!(PatchInset, UiRect, |patch, v| node(patch, |n| {
    n.left = v.left;
    n.right = v.right;
    n.top = v.top;
    n.bottom = v.bottom;
}));

field_patch!(PatchBackground, Color, |patch, v| {
    patch.insert(BackgroundColor(*v));
});

field_patch!(PatchBorderColor, Color, |patch, v| {
    patch.insert(BorderColor::all(*v));
});

field_patch!(PatchZIndex, i32, |patch, v| {
    patch.insert(GlobalZIndex(*v));
});

// `None` drops the override, leaving the node in its parent's stack.
field_patch!(PatchOptionalZ, Option<i32>, |patch, v| match v {
    Some(z) => {
        patch.insert(GlobalZIndex(*z));
    }
    None => {
        patch.remove::<GlobalZIndex>();
    }
});

// ---------------------------------------------------------------------
// shared, but shaped by one element's layout
// ---------------------------------------------------------------------

// A square icon: one value drives both dimensions.
field_patch!(PatchIconSize, Val, |patch, v| node(patch, |n| {
    n.width = *v;
    n.height = *v;
}));

// An overlay never blocks; what it catches, it catches by being seen.
field_patch!(PatchOverlayPickable, bool, |patch, v| {
    patch.insert(Pickable {
        should_block_lower: false,
        is_hoverable: *v,
    });
});

field_patch!(PatchPanelScroll, f32, |patch, v| {
    with::<ScrollPosition>(patch, |pos| pos.0.y = *v);
});

field_patch!(PatchPanelScrolls, bool, |patch, v| {
    let overflow = if *v {
        Overflow::scroll_y()
    } else {
        Overflow::clip()
    };
    node(patch, move |n| n.overflow = overflow);
});

fn axis(on: bool) -> OverflowAxis {
    if on {
        OverflowAxis::Scroll
    } else {
        OverflowAxis::Visible
    }
}

field_patch!(PatchScrollX, bool, |patch, v| {
    let a = axis(*v);
    node(patch, move |n| n.overflow.x = a);
});

field_patch!(PatchScrollY, bool, |patch, v| {
    let a = axis(*v);
    node(patch, move |n| n.overflow.y = a);
});

// A time-axis label's `left`, plus the centring its position implies:
// the leftmost mark reads flush, the rest centre over their tick.
field_patch!(PatchTimeLabelX, Val, |patch, v| {
    let x = *v;
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
});

// A timeline track, sized to its duration on both `width` and its
// own min.
field_patch!(PatchTrackWidth, Val, |patch, v| {
    let width = *v;
    node(patch, move |n| {
        n.width = width;
        n.min_width = width;
    });
});

// A timeline clip's border thickens when it is selected.
field_patch!(PatchSelected, bool, |patch, v| {
    let w = if *v { 2 } else { 1 };
    node(patch, move |n| n.border = UiRect::all(px(w)));
});

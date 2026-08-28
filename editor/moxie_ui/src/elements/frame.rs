use crate::reactive::BevyHost;
use bevy::prelude::*;
use bevy_fynix::WorldEntityMut;
use fynix::element::{Element, ElementVisual};
use fynix::ui::{Build, Patch};

/// The sized, optionally filled container almost every other widget's
/// root node turns out to be.
#[derive(Element)]
pub struct Frame {
    pub width: Val,
    pub height: Val,
    /// The floor `width`/`height` can shrink to. `Auto` floors a
    /// flex item at its own content's size, which can fight a
    /// percentage `width` above trying to shrink it further; `px(0)`
    /// lifts that floor.
    pub min_width: Val,
    pub min_height: Val,
    /// Absolute for a frame that places itself, with [`inset`] saying
    /// where.
    ///
    /// [`inset`]: Frame::inset
    pub position: PositionType,
    /// How far each edge sits from the parent's, for an absolute
    /// frame. `Auto` on an edge leaves that one to the layout.
    #[default(UiRect::all(auto()))]
    pub inset: UiRect,
    pub direction: FlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub align: AlignItems,
    pub justify: JustifyContent,
    pub padding: UiRect,
    pub margin: UiRect,
    pub overflow: Overflow,
    /// Between rows, and between columns: a row of things wants the
    /// second, a column the first.
    #[default(Val::ZERO)]
    pub row_gap: Val,
    #[default(Val::ZERO)]
    pub column_gap: Val,
    #[default(Val::ZERO)]
    pub radius: Val,
    /// Transparent by default, for a frame that only wants the
    /// layout.
    #[default(Color::NONE)]
    pub background: Color,
    /// `None` hides it, which is how a frame that depends on a size it
    /// has not measured yet avoids showing up at its intrinsic size
    /// for a frame.
    pub display: Display,
    /// Where it sits in the window's stack, for a frame that has to
    /// be above what it does not sit inside. `None` leaves it in its
    /// parent's, where the tree already puts it.
    pub z: Option<i32>,
}

impl Frame {
    fn node(&self) -> Node {
        Node {
            width: self.width,
            height: self.height,
            min_width: self.min_width,
            min_height: self.min_height,
            position_type: self.position,
            left: self.inset.left,
            right: self.inset.right,
            top: self.inset.top,
            bottom: self.inset.bottom,
            flex_direction: self.direction,
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            align_items: self.align,
            justify_content: self.justify,
            padding: self.padding,
            margin: self.margin,
            overflow: self.overflow,
            row_gap: self.row_gap,
            column_gap: self.column_gap,
            border_radius: BorderRadius::all(self.radius),
            display: self.display,
            ..default()
        }
    }
}

impl Frame {
    /// Written whole, so that a frame going back into its parent's
    /// stack loses the component rather than keeping a stale one.
    fn stack(&self, entity: &mut impl WorldEntityMut) {
        match self.z {
            Some(z) => entity.insert(GlobalZIndex(z)),
            None => entity.remove::<GlobalZIndex>(),
        };
    }
}

impl ElementVisual<BevyHost> for Frame {
    fn build_fields(&self, build: &mut Build<BevyHost, Self>) {
        build.insert((self.node(), BackgroundColor(self.background)));

        self.stack(build);
    }

    fn patch_fields(
        &self,
        patch: &mut Patch<BevyHost>,
        field: FrameField,
    ) {
        match field {
            FrameField::Background => {
                patch.insert(BackgroundColor(self.background));
            }
            // Every other field is one of `Node`'s, and writing the
            // node whole is one insert rather than eight arms that
            // each write a field of it.
            FrameField::Z => self.stack(patch),
            FrameField::Width
            | FrameField::Height
            | FrameField::MinWidth
            | FrameField::MinHeight
            | FrameField::Position
            | FrameField::Inset
            | FrameField::FlexGrow
            | FrameField::FlexShrink
            | FrameField::Direction
            | FrameField::Align
            | FrameField::Justify
            | FrameField::Padding
            | FrameField::Margin
            | FrameField::Overflow
            | FrameField::RowGap
            | FrameField::ColumnGap
            | FrameField::Radius
            | FrameField::Display => {
                patch.insert(self.node());
            }
        }
    }
}

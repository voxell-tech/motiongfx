use bevy::prelude::*;
use bevy_fynix::BevyUi;
use bevy_fynix::host::BevyHost;
use fynix_mock::OverrideDefault;
use fynix_mock::element::{Element, ElementVisual};
use fynix_mock::lenz::Lenz;

/// The sized, optionally filled container almost every other widget's
/// root node turns out to be.
#[derive(Element, OverrideDefault, Lenz)]
pub struct Frame {
    pub width: Val,
    pub height: Val,
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
    fn stack(&self, world: &mut World, node: Entity) {
        let mut entity = world.entity_mut(node);

        match self.z {
            Some(z) => entity.insert(GlobalZIndex(z)),
            None => entity.remove::<GlobalZIndex>(),
        };
    }
}

impl ElementVisual<BevyHost> for Frame {
    fn build_fields(&self, ui: &mut BevyUi<'_>) {
        let node = ui.parent();
        let world = &mut *ui.world;

        world
            .entity_mut(node)
            .insert((self.node(), BackgroundColor(self.background)));
        self.stack(world, node);
    }

    fn patch_fields(
        &self,
        world: &mut World,
        node: Entity,
        field: FrameField,
    ) {
        match field {
            FrameField::Background => {
                world
                    .entity_mut(node)
                    .insert(BackgroundColor(self.background));
            }
            // Every other field is one of `Node`'s, and writing the
            // node whole is one insert rather than eight arms that
            // each write a field of it.
            FrameField::Z => self.stack(world, node),
            FrameField::Width
            | FrameField::Height
            | FrameField::Position
            | FrameField::Inset
            | FrameField::FlexGrow
            | FrameField::FlexShrink
            | FrameField::Direction
            | FrameField::Align
            | FrameField::Justify
            | FrameField::Padding
            | FrameField::RowGap
            | FrameField::ColumnGap
            | FrameField::Radius
            | FrameField::Display => {
                world.entity_mut(node).insert(self.node());
            }
        }
    }
}

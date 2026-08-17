//! Folding something away.
//!
//! The state is a marker on the node that heads the fold, so it is
//! read and written right where it is used and nothing has to name
//! it. It goes when that node does: rebuilding what surrounds a fold
//! opens it again.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::ElementMutExt;
use fynix_mock::composer::Composer;
use fynix_mock::style::StyledElem;
use fynix_mock::ui::{ElementHandle, ElementMut};
use fynix_mock::{elem, val};

use crate::elements::{
    ButtonElem, ButtonElemCursor, Frame, FrameCursor, GhostButton,
    Icon, IconCursor,
};
use crate::icons;
use crate::reactive::{BevyHost, BevyUi, component_changed_on};
use crate::theme::EditorTheme;

/// The chevron's rotation, clockwise from the asset's resting
/// up-pointing orientation: right when shut, down when open.
pub const CHEVRON_SHUT: f32 = 90.0;
pub const CHEVRON_OPEN: f32 = 180.0;

/// A chevron of its own, sized to sit beside a row.
const TOGGLE: f32 = 14.0;

/// How far the rail sets the body in from the header.
const INDENT: f32 = 9.0;

/// On a [`Foldable`]'s own node while its body is hidden.
#[derive(Component)]
pub struct Folded;

pub fn is_folded(world: &World, node: Entity) -> bool {
    world.get::<Folded>(node).is_some()
}

pub fn toggle_folded(world: &mut World, node: Entity) {
    let Ok(mut node) = world.get_entity_mut(node) else {
        return;
    };

    if node.contains::<Folded>() {
        node.remove::<Folded>();
    } else {
        node.insert(Folded);
    }
}

/// What a click has to land on to fold.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// The header itself, turning the chevron already in its icon
    /// slot - for a header that has nothing else to mean.
    Header,
    /// A chevron of its own beside the header, leaving the header
    /// free to mean something else, like selecting the row.
    Chevron,
}

/// A header that folds away what is built under it.
///
/// The header is passed in whole rather than described, so a section
/// and a tree row can look nothing alike and still fold the same way.
/// All this owns is the mechanism: the click that toggles, the
/// chevron that turns, the body that goes, and the rail marking how
/// deep that body sits.
pub struct Foldable<
    S,
    B,
    H = for<'u, 'a> fn(ElementMut<'u, 'a, BevyHost, ButtonElem>),
> {
    /// Anything built on a [`ButtonElem`]. Under [`Toggle::Header`]
    /// its icon slot is the chevron, so it has to carry one.
    pub header: S,
    pub toggle: Toggle,
    /// Whether there is anything to fold. A header with nothing under
    /// it neither turns nor toggles, and its chevron is left out
    /// rather than dimmed, since a hover would light it up again.
    pub enabled: bool,
    /// Run on the header once it's built, after folding is wired to
    /// it under [`Toggle::Header`] - so a header that also means
    /// something of its own, like selecting a row, can still say so.
    pub on_header: H,
    pub body: B,
}

impl<S, B, H> Composer<BevyHost> for Foldable<S, B, H>
where
    S: StyledElem<Host = BevyHost, Element = ButtonElem>,
    B: FnOnce(&mut BevyUi),
    H: for<'u, 'a> FnOnce(ElementMut<'u, 'a, BevyHost, ButtonElem>),
{
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self {
            header,
            toggle,
            enabled,
            on_header,
            body,
        } = self;
        let theme = ui.world.resource::<EditorTheme>().clone();
        let chevron = enabled && toggle == Toggle::Chevron;

        let root = ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column,
            row_gap = px(2)
        ));
        // Every part reads the fold off this one node, so the chevron
        // can turn and the body can go.
        let node = root.id();

        root.with(move |ui| {
            ui.elem(elem!(
                Frame,
                width = percent(100),
                direction = FlexDirection::Row,
                align = AlignItems::Center
            ))
            .with(move |ui| {
                if chevron {
                    let toggle = ui.elem(elem!(
                        !GhostButton,
                        width = px(TOGGLE),
                        height = px(TOGGLE),
                        radius = px(3),
                        icon = val!(
                            Icon,
                            image = icons::CHEVRON,
                            size = px(8),
                            color = theme.text_muted,
                            rotation = CHEVRON_OPEN
                        )
                    ));
                    folds(toggle, node);
                }

                // Takes the rest of the row, so a header asking for
                // its full width gets what is left beside a chevron.
                ui.elem(elem!(Frame, flex_grow = 1.0f32)).with(
                    move |ui| {
                        let mut header = ui.elem(header);
                        if enabled && !chevron {
                            header = folds(header, node);
                        }
                        on_header(header);
                    },
                );
            });

            // Set in under the chevron's own middle, so the rail runs
            // down through it.
            ui.elem(elem!(
                Frame,
                width = percent(100),
                direction = FlexDirection::Row,
                align = AlignItems::Stretch,
                padding = UiRect::new(
                    px(TOGGLE / 2.0),
                    Val::ZERO,
                    Val::ZERO,
                    Val::ZERO
                )
            ))
            .bind(
                |frame| frame.display(),
                component_changed_on::<Folded>(node),
                move |world, _| {
                    if is_folded(world, node) {
                        Display::None
                    } else {
                        Display::Flex
                    }
                },
            )
            .with(move |ui| {
                // A rail beside the body, the way a tree view marks
                // how deep a nested branch sits - stretched to the
                // block's height rather than sized by hand.
                ui.elem(elem!(
                    Frame,
                    width = px(1),
                    background = theme.palette.base[2]
                ));
                ui.elem(elem!(
                    Frame,
                    direction = FlexDirection::Column,
                    flex_grow = 1.0f32,
                    padding = UiRect::new(
                        px(INDENT),
                        Val::ZERO,
                        Val::ZERO,
                        Val::ZERO
                    )
                ))
                .with(body);
            });
        })
        .handle()
    }
}

/// Makes `button` the one that folds `node`, turning its chevron with
/// the state.
fn folds<'u, 'a>(
    button: ElementMut<'u, 'a, BevyHost, ButtonElem>,
    node: Entity,
) -> ElementMut<'u, 'a, BevyHost, ButtonElem> {
    button
        .observe(move |_: On<Activate>, mut commands: Commands| {
            commands.queue(move |world: &mut World| {
                toggle_folded(world, node);
            });
        })
        .bind(
            |button| button.icon().rotation(),
            component_changed_on::<Folded>(node),
            move |world, _| {
                if is_folded(world, node) {
                    CHEVRON_SHUT
                } else {
                    CHEVRON_OPEN
                }
            },
        )
}

//! Folding something away.
//!
//! The chevron and body key off a marker on the row's own node, but
//! that node gets despawned and rebuilt on a list-level change. So
//! [`Foldable::open`]/`on_toggle` let the caller keep that state
//! somewhere that survives, such as a component on the entity a row
//! stands for.
//!
//! The body builds lazily, only the first time a row opens, so a fold
//! over something expensive (a filesystem read, say) never pays for
//! what nobody has looked at.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::WorldEntityMut;
use fynix::WorldNodeRef;
use fynix::composer::Composer;
use fynix::records::BuildFn;
use fynix::style::StyledElem;
use fynix::ui::{ElementHandle, ElementMut};
use fynix::{elem, val};

use crate::elements::{
    ButtonElem, ButtonElemCursor, Frame, FrameCursor, Icon,
    IconCursor, TintButton,
};
use crate::icons;
use crate::reactive::{BevyHost, BevyUi, component_changed_on};

/// The chevron's rotation, clockwise from the asset's resting
/// up-pointing orientation. Right when shut, down when open.
pub const CHEVRON_SHUT: f32 = 90.0;
pub const CHEVRON_OPEN: f32 = 180.0;

/// The rail's own width, one level of a body's indent.
pub const RAIL_WIDTH: f32 = 1.0;

/// On a [`Foldable`]'s own node while its body is hidden. Private to
/// this row's own reactivity. A caller after something that survives
/// this node being rebuilt wants [`Foldable::open`]/`on_toggle`
/// instead.
#[derive(Component)]
struct Folded;

fn is_folded(world: &World, node: Entity) -> bool {
    world.get::<Folded>(node).is_some()
}

fn toggle_folded(world: &mut World, node: Entity) {
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
pub enum FoldsOn {
    /// The header itself, turning the chevron already in its icon
    /// slot. For a header that has nothing else to mean.
    Header,
    /// A chevron of its own beside the header, leaving the header
    /// free to mean something else, like selecting the row.
    Chevron,
}

/// A header that folds away what is built under it.
///
/// The header is passed in whole rather than described, so a section
/// and a tree row can look nothing alike and still fold the same way.
/// All this owns is the click that toggles, the chevron that turns,
/// the body that goes, and the rail marking how deep that body sits.
pub struct Foldable<
    S: StyledElem<Host = BevyHost, Element = ButtonElem>,
    B: BuildFn<BevyHost>,
    H: for<'u, 'a> FnOnce(ElementMut<'u, 'a, BevyHost, ButtonElem>),
    T: Fn(&mut World, bool) + Clone + Send + Sync + 'static,
> {
    /// Anything built on a [`ButtonElem`]. Under [`FoldsOn::Header`]
    /// its icon slot is the chevron, so it has to carry one.
    pub header: S,
    pub folds_on: FoldsOn,
    /// Whether there is anything to fold. A header with nothing under
    /// it neither turns nor toggles, and its chevron is left out
    /// rather than dimmed, since a hover would light it up again.
    pub enabled: bool,
    /// Run on the header once it's built, after folding is wired to
    /// it under [`FoldsOn::Header`], so a header that also means
    /// something of its own, like selecting a row, can still say so.
    pub on_header: H,
    pub body: B,
    /// Whatever this row was last left as, from wherever the caller
    /// keeps that. This row's own node carries nothing across a
    /// rebuild of the list around it.
    pub open: bool,
    /// Mirrors a toggle into the caller's own store. Where that lives
    /// is entirely the caller's call. A component on the entity a row
    /// stands for cleans itself up when the entity does, and nothing
    /// here needs to know either way.
    pub on_toggle: T,
}

impl<S, B, H, T> Composer<BevyHost> for Foldable<S, B, H, T>
where
    S: StyledElem<Host = BevyHost, Element = ButtonElem>,
    B: BuildFn<BevyHost>,
    H: for<'u, 'a> FnOnce(ElementMut<'u, 'a, BevyHost, ButtonElem>),
    T: Fn(&mut World, bool) + Clone + Send + Sync + 'static,
{
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self {
            header,
            folds_on,
            enabled,
            on_header,
            body,
            open,
            on_toggle,
        } = self;

        let muted = ui.theme.text_muted;
        let toggle_size = ui.theme.fold_toggle;
        let indent = ui.theme.fold_indent;
        let chevron = enabled && folds_on == FoldsOn::Chevron;

        let mut root = ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Column,
            row_gap = px(2)
        ));
        // Every part reads the fold off this one node, so the chevron
        // can turn and the body can go. A fresh node takes whatever
        // `open` says, not always closed.
        let node = root.id();
        if !open {
            root.insert(Folded);
        }

        root.with(move |ui| {
            ui.elem(elem!(
                Frame,
                width = percent(100),
                direction = FlexDirection::Row,
                align = AlignItems::Center
            ))
            .with(move |ui| {
                if chevron {
                    let mut toggle = ui.elem(elem!(
                        !TintButton::default(),
                        width = px(toggle_size),
                        height = px(toggle_size),
                        radius = px(3),
                        icon = val!(
                            Icon,
                            image = icons::CHEVRON,
                            size = px(8),
                            color = muted,
                            rotation = CHEVRON_OPEN
                        )
                    ));
                    folds(&mut toggle, node, on_toggle.clone());
                }

                // Takes the rest of the row, so a header asking for
                // its full width gets what is left beside a chevron.
                ui.elem(elem!(Frame, flex_grow = 1.0f32)).with(
                    move |ui| {
                        let mut header = ui.elem(header);
                        if enabled && !chevron {
                            folds(&mut header, node, on_toggle);
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
                    px(toggle_size / 2.0),
                    Val::ZERO,
                    Val::ZERO,
                    Val::ZERO
                )
            ))
            .bind(
                |frame| frame.display(),
                component_changed_on::<Folded>(node),
                move |WorldNodeRef { world, .. }| {
                    if is_folded(world, node) {
                        Display::None
                    } else {
                        Display::Flex
                    }
                },
            )
            .with(move |ui| {
                // The rail. Stretched to the block's height, not
                // sized by hand.
                ui.elem(elem!(
                    Frame,
                    width = px(RAIL_WIDTH),
                    background = ui.theme.palette.base[2]
                ));
                ui.elem(elem!(
                    Frame,
                    direction = FlexDirection::Column,
                    flex_grow = 1.0f32,
                    padding = UiRect::new(
                        px(indent),
                        Val::ZERO,
                        Val::ZERO,
                        Val::ZERO
                    )
                ))
                .watch(
                    component_changed_on::<Folded>(node),
                    move |ui| {
                        // Stays empty while folded, rather than
                        // building what nothing has opened yet.
                        if is_folded(ui.world, node) {
                            return;
                        }
                        body(ui);
                    },
                );
            });
        })
        .handle()
    }
}

/// Makes `button` the one that folds `node`, turning its chevron with
/// the state and mirroring the result through `on_toggle`.
fn folds<T>(
    button: &mut ElementMut<BevyHost, ButtonElem>,
    node: Entity,
    on_toggle: T,
) where
    T: Fn(&mut World, bool) + Clone + Send + Sync + 'static,
{
    button
        .observe(move |_: On<Activate>, mut commands: Commands| {
            let on_toggle = on_toggle.clone();
            commands.queue(move |world: &mut World| {
                toggle_folded(world, node);
                on_toggle(world, !is_folded(world, node));
            });
        })
        .bind(
            |button| button.icon().rotation(),
            component_changed_on::<Folded>(node),
            move |WorldNodeRef { world, .. }| {
                if is_folded(world, node) {
                    CHEVRON_SHUT
                } else {
                    CHEVRON_OPEN
                }
            },
        );
}

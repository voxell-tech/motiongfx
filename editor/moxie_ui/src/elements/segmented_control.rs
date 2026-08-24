//! A row of mutually exclusive options, as a composer rather than an
//! element - it builds a subtree of [`ButtonElem`]s, none of it kept
//! around to be patched later.

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::EntityExt as _;
use fynix_mock::composer::Composer;
use fynix_mock::ui::ElementHandle;
use fynix_mock::{elem, val};

use super::{Frame, Label, SegmentButton};
use crate::reactive::{BevyHost, BevyUi};

/// A 3-way (or more) radio, one option filled solid - bevy_feathers'
/// own `RoundedCorners`/`ButtonVariant::Primary` pattern, rounded only
/// at the row's own ends (the row clips its children rather than
/// rounding each segment) with a 1px gap as the seam between segments,
/// not a divider line.
pub struct SegmentedControl<F> {
    pub options: Vec<String>,
    pub selected: usize,
    /// `Commands` to queue a world write with, since picking a segment
    /// only fires from inside its own `Activate` observer.
    pub on_select: F,
}

impl<F> Composer<BevyHost> for SegmentedControl<F>
where
    F: Fn(usize, &mut Commands) + Clone + Send + Sync + 'static,
{
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self {
            options,
            selected,
            on_select,
        } = self;
        let theme = ui.theme;

        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Row,
            column_gap = px(1),
            radius = px(4),
            overflow = Overflow::clip()
        ))
        .with(move |ui| {
            for (i, label) in options.into_iter().enumerate() {
                let active = i == selected;
                let text_color = if active {
                    theme.palette.base[0]
                } else {
                    theme.text_primary
                };
                let on_select = on_select.clone();

                ui.elem(elem!(
                    !SegmentButton { active },
                    label = val!(
                        Label,
                        text = label,
                        size = 11.0f32,
                        bold = active,
                        wrap = false,
                        color = Some(text_color)
                    )
                ))
                .observe(
                    move |_: On<Activate>, mut commands: Commands| {
                        on_select(i, &mut commands);
                    },
                );
            }
        })
        .handle()
    }
}

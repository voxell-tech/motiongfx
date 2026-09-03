//! A row of mutually exclusive options, as a composer rather than an
//! element - it builds a subtree of [`Button`]s, none of it kept
//! around to be patched later.
//!
//! [`Button`]: crate::elements::button::Button

use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy_fynix::WorldEntityMut as _;
use fynix::composer::Composer;
use fynix::elem;
use fynix::ui::ElementHandle;

use super::{Frame, Label, SegmentButton};
use crate::reactive::{BevyUi, FynixHost};

/// A 3-way (or more) radio, one option filled solid - bevy_feathers'
/// own `RoundedCorners`/`ButtonVariant::Primary` pattern: only the
/// row's own two ends are rounded, each segment its own corners rather
/// than the row clipping a straight-edged strip, with a 1px gap as the
/// seam between segments, not a divider line.
pub struct SegmentedControl<F> {
    pub options: Vec<String>,
    pub selected: usize,
    /// `Commands` to queue a world write with, since picking a segment
    /// only fires from inside its own `Activate` observer.
    pub on_select: F,
}

impl<F> Composer<FynixHost> for SegmentedControl<F>
where
    F: Fn(usize, &mut Commands) + Clone + Send + Sync + 'static,
{
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
        let Self {
            options,
            selected,
            on_select,
        } = self;
        let theme = ui.theme;

        let count = options.len();

        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Row,
            column_gap = px(1)
        ))
        .with(move |ui| {
            for (i, label) in options.into_iter().enumerate() {
                let active = i == selected;
                let text_color = if active {
                    theme.palette.base[0]
                } else {
                    theme.text_primary
                };
                let corners = segment_corners(i, count);
                let on_select = on_select.clone();

                ui.elem(elem!(
                    !SegmentButton { active },
                    corners = Some(corners),
                    label = elem!(
                        Label,
                        text = label,
                        size = 11.0f32,
                        bold = active,
                        wrap = false,
                        color = text_color
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

/// The corner radius for the segment at `index` of `count`: rounded on
/// whichever side (or both, or neither) sits at the row's own edge.
/// Not themed yet - see the backlog entry to give `EditorTheme` a
/// button corner radius and use it here and everywhere else a button
/// rounds itself.
fn segment_corners(index: usize, count: usize) -> BorderRadius {
    let radius = px(4);
    let first = index == 0;
    let last = index == count - 1;

    BorderRadius::new(
        if first { radius } else { Val::ZERO },
        if last { radius } else { Val::ZERO },
        if last { radius } else { Val::ZERO },
        if first { radius } else { Val::ZERO },
    )
}

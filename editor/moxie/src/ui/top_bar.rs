//! The menu bar above the dock, holding what acts on the project as a
//! whole rather than on anything a panel is showing.

use bevy::prelude::*;
use bevy::ui_widgets::{
    Activate, ActivateOnPress, MenuButton as MenuButtonBehavior,
};
use bevy_fynix::WorldEntityMut;
use fynix::composer::Composer;
use fynix::ui::ElementHandle;
use fynix::{elem, val};
use moxie_ui::elements::{
    Dropdown, DropdownItem, DropdownItemCursor, DropdownList,
    DropdownMenu, Frame, Label, MenuButton,
};
use moxie_ui::motion::MotionExt;
use moxie_ui::reactive::{BevyUi, FynixHost};
use moxie_ui::theme::EditorTheme;

use crate::project;

pub(super) struct TopBar;

impl Composer<FynixHost> for TopBar {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, Frame> {
        ui.elem(elem!(
            Frame,
            width = percent(100),
            direction = FlexDirection::Row,
            align = AlignItems::Center,
        ))
        .with(|ui| {
            ui.compose(Menu {
                name: "File",
                entries: vec![
                    ("Open", project::load_scene),
                    ("Save", project::save_scene),
                ],
            });
        })
        .handle()
    }
}

/// One menu: the name in the bar, and what picking an entry runs.
struct Menu {
    name: &'static str,
    entries: Vec<(&'static str, fn(&mut World))>,
}

impl Composer<FynixHost> for Menu {
    type Element = DropdownMenu;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<FynixHost, DropdownMenu> {
        let Self { name, entries } = self;
        let theme = ui.theme;
        // Sized to the longest entry, so the list clears its own text
        // whichever menu it belongs to.
        let width = Dropdown::width_for(
            &entries
                .iter()
                .map(|(entry, _)| entry.to_string())
                .collect::<Vec<_>>(),
            12.0,
        );

        ui.elem(elem!(DropdownMenu))
            .with(move |ui| {
                title(ui, theme, name);

                ui.elem(elem!(
                    DropdownList,
                    width = width,
                    radius = Val::ZERO
                ))
                .with(move |ui| {
                    for (entry, run) in entries {
                        item(ui, theme, entry, run);
                    }
                });
            })
            .handle()
    }
}

/// The name in the bar, which opens the menu.
///
/// A button rather than a [`Dropdown`]: an entry in a menu bar is a
/// word, not a form control, so it wears no chevron.
fn title(ui: &mut BevyUi, theme: &EditorTheme, name: &str) {
    ui.elem(elem!(
        !MenuButton,
        label = val!(
            Label,
            text = name.to_string(),
            wrap = false,
            color = theme.text_primary
        )
    ))
    // What the menu's own observer reaches this through to open the
    // list beneath it.
    .insert((MenuButtonBehavior, ActivateOnPress));
}

/// One row of the open menu. Picking it closes the list, and runs
/// `run` once the click's own commands have been applied.
fn item(
    ui: &mut BevyUi,
    theme: &EditorTheme,
    entry: &str,
    run: fn(&mut World),
) {
    ui.elem(elem!(
        DropdownItem,
        radius = Val::ZERO,
        label = val!(
            Label,
            text = entry.to_string(),
            wrap = false,
            color = theme.text_primary
        )
    ))
    .lit(|item| item.fill(), theme.hover_overlay, theme.hover_overlay)
    .observe(move |_: On<Activate>, mut commands: Commands| {
        commands.queue(run);
    });
}

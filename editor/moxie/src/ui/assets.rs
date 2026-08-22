//! Bookmarked-folder asset browser.
//!
//! Bookmarks and the folders under them expand in place, the same
//! [`Foldable`] the hierarchy panel builds its own tree from - a
//! chevron per row, an indent rail marking depth, and a body read
//! only the first time a row opens, since unlike an entity's children
//! a folder's are a real filesystem read.
//!
//! A recognized file can be dragged onto an inspector's `Handle<T>`
//! field; see `moxie_ui::asset`.

use std::collections::BTreeSet;
use std::fs;
use std::ops::Bound;
use std::path::{Path, PathBuf};

use bevy::feathers::cursor::EntityCursor;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;
use bevy::window::SystemCursorIcon;
use bevy_fynix::EntityExt;
use fynix_mock::composer::Composer;
use fynix_mock::ui::{ElementHandle, ElementMut};
use fynix_mock::{elem, val};
use moxie_asset::AssetKinds;
use moxie_ui::asset::draggable;
use moxie_ui::elements::{
    ButtonElem, Frame, Icon, Label, Panel, ScrollArea, TintButton,
};
use moxie_ui::fold::{CHEVRON_SHUT, Foldable, FoldsOn};
use moxie_ui::reactive::{BevyHost, BevyUi, resource_changed};

use super::PANEL_PADDING;
use crate::{ProjectBookmarks, ProjectPath};

/// Room below the last row for the button that floats over it.
const BUTTON_CLEARANCE: f32 = 34.0;

/// Which folders were left open, keyed by path, since nothing else
/// could hold this. A `BTreeSet`, since path components sort a
/// subtree into one run, so [`prune_fold_state`]/[`remove_bookmark`]
/// can drop it as a bounded range instead of walking every open
/// folder.
#[derive(Resource, Default)]
pub(super) struct AssetFoldState(BTreeSet<PathBuf>);

pub(super) struct AssetsPanel;

impl Composer<BevyHost> for AssetsPanel {
    type Element = Panel;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Panel> {
        ui.elem(elem!(Panel))
            .with(|ui| {
                ui.compose(Listing);
                ui.compose(AddButton);
            })
            .handle()
    }
}

/// Floated over the corner, not given a strip of its own, so it stays
/// put however far the list is scrolled.
struct AddButton;

impl Composer<BevyHost> for AddButton {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        ui.elem(elem!(
            Frame,
            position = PositionType::Absolute,
            inset = UiRect::new(
                auto(),
                px(PANEL_PADDING),
                auto(),
                px(PANEL_PADDING)
            )
        ))
        .with(move |ui| {
            ui.elem(elem!(
                !TintButton::default(),
                icon = val!(Icon, image = crate::icons::PLUS)
            ))
            .observe(
                |_: On<Activate>, mut commands: Commands| {
                    commands.queue(add_bookmark);
                },
            );
        })
        .handle()
    }
}

/// Prompts for a folder and bookmarks it. Dropped silently if the
/// dialog was dismissed, or if it nests with a bookmark already
/// there in either direction, since the row it would open into and
/// the row already showing it would otherwise both list the same
/// files.
fn add_bookmark(world: &mut World) {
    let Some(folder) = rfd::FileDialog::new().pick_folder() else {
        return;
    };

    let mut bookmarks = world.resource_mut::<ProjectBookmarks>();
    let nests = bookmarks.0.iter().any(|existing| {
        folder.starts_with(existing) || existing.starts_with(&folder)
    });
    if nests {
        return;
    }
    bookmarks.0.push(folder);
}

struct Listing;

impl Composer<BevyHost> for Listing {
    type Element = ScrollArea;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, ScrollArea> {
        ui.elem(elem!(
            ScrollArea,
            width = percent(100),
            flex_grow = 1.0f32,
            padding = UiRect::new(
                px(PANEL_PADDING),
                px(PANEL_PADDING),
                px(PANEL_PADDING),
                px(BUTTON_CLEARANCE)
            ),
            scroll_x = false
        ))
        .watch(bookmarks_or_project_changed(), build_bookmarks)
        .handle()
    }
}

/// Fires on either resource, since [`build_bookmarks`] draws from
/// both.
fn bookmarks_or_project_changed()
-> impl FnMut(&World, Entity) -> bool + Send + Sync + 'static {
    let mut bookmarks = resource_changed::<ProjectBookmarks>();
    let mut project = resource_changed::<ProjectPath>();
    move |world, node| {
        let a = bookmarks(world, node);
        let b = project(world, node);
        a || b
    }
}

fn build_bookmarks(ui: &mut BevyUi) {
    let project = ui
        .world
        .resource::<ProjectPath>()
        .0
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    if let Some(path) = project {
        ui.compose(BookmarkRow { index: None, path });
    }

    let paths = ui.world.resource::<ProjectBookmarks>().0.clone();
    for (index, path) in paths.into_iter().enumerate() {
        ui.compose(BookmarkRow {
            index: Some(index),
            path,
        });
    }
}

/// One bookmark: its folder name, expanding its contents in place,
/// and, unless it's the open project's own folder, a button to drop
/// it.
struct BookmarkRow {
    index: Option<usize>,
    path: PathBuf,
}

impl Composer<BevyHost> for BookmarkRow {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let Self { index, path } = self;
        let name = display_name(&path);
        let enabled = has_entries(&path);
        let open =
            ui.world.resource::<AssetFoldState>().0.contains(&path);
        let toggle_path = path.clone();

        let muted = ui.theme.text_muted;
        let primary = ui.theme.text_primary;

        ui.compose(Foldable {
            header: elem!(
                !TintButton::default(),
                width = percent(100),
                justify = JustifyContent::FlexStart,
                icon = val!(
                    Icon,
                    image = moxie_ui::icons::CHEVRON,
                    color = muted,
                    rotation = CHEVRON_SHUT
                ),
                label = val!(
                    Label,
                    text = name.clone(),
                    color = Some(primary),
                    wrap = false
                )
            ),
            folds_on: FoldsOn::Header,
            enabled,
            on_header: move |mut header: ElementMut<BevyHost, ButtonElem>| {
                // Nothing to drop for the project's own folder.
                let Some(index) = index else {
                    return;
                };

                // A delete button beside the label, injected as an
                // extra child rather than one of `ButtonElem`'s own
                // icon/label slots. It takes its own click for
                // itself, so the header's fold never hears it.
                header.with(move |ui| {
                    // Eats whatever room icon and label leave, so
                    // the delete button lands flush against the far
                    // end.
                    ui.elem(elem!(Frame, flex_grow = 1.0f32));

                    ui.elem(elem!(
                        !TintButton {
                            tint: Some(ui.theme.critical)
                        },
                        width = px(14),
                        height = px(14),
                        padding = UiRect::ZERO,
                        radius = px(2),
                        icon = val!(
                            Icon,
                            image = crate::icons::TRASH,
                            size = px(10)
                        )
                    ))
                    .observe(
                        move |_: On<Activate>, mut commands: Commands| {
                            commands.queue(remove_bookmark(index));
                        },
                    );
                });
            },
            body: move |ui: &mut BevyUi| {
                build_children(ui, &path);
            },
            open,
            on_toggle: move |world: &mut World, open: bool| {
                let mut state = world.resource_mut::<AssetFoldState>();
                if open {
                    state.0.insert(toggle_path.clone());
                } else {
                    state.0.remove(&toggle_path);
                }
            },
        })
        .handle()
    }
}

/// Drops the bookmark at `index`, and every fold state key under the
/// folder it named. A subfolder's key would otherwise sit unread
/// forever, since nothing else names it once the bookmark is gone.
fn remove_bookmark(index: usize) -> impl FnOnce(&mut World) {
    move |world: &mut World| {
        let mut bookmarks = world.resource_mut::<ProjectBookmarks>();
        if index >= bookmarks.0.len() {
            return;
        }
        let removed = bookmarks.0.remove(index);

        let mut state = world.resource_mut::<AssetFoldState>();
        let gone = state
            .0
            .range(removed.clone()..)
            .take_while(|path| path.starts_with(&removed))
            .cloned()
            .collect::<Vec<_>>();
        for path in gone {
            state.0.remove(&path);
        }
    }
}

/// One subfolder: its name, expanding its own contents in place.
struct FolderRow {
    path: PathBuf,
}

impl Composer<BevyHost> for FolderRow {
    type Element = Frame;

    fn compose(
        self,
        ui: &mut BevyUi,
    ) -> ElementHandle<BevyHost, Frame> {
        let path = self.path;
        let name = display_name(&path);
        let enabled = has_entries(&path);
        let open =
            ui.world.resource::<AssetFoldState>().0.contains(&path);
        let toggle_path = path.clone();

        let muted = ui.theme.text_muted;
        let primary = ui.theme.text_primary;

        ui.compose(Foldable {
            header: elem!(
                !TintButton::default(),
                justify = JustifyContent::FlexStart,
                icon = val!(
                    Icon,
                    image = moxie_ui::icons::CHEVRON,
                    color = muted,
                    rotation = CHEVRON_SHUT
                ),
                label = val!(
                    Label,
                    text = name.clone(),
                    color = Some(primary),
                    wrap = false
                )
            ),
            folds_on: FoldsOn::Header,
            enabled,
            on_header: |_: ElementMut<BevyHost, ButtonElem>| {},
            body: move |ui: &mut BevyUi| {
                build_children(ui, &path);
            },
            open,
            on_toggle: move |world: &mut World, open: bool| {
                let mut state =
                    world.resource_mut::<AssetFoldState>();
                if open {
                    state.0.insert(toggle_path.clone());
                } else {
                    state.0.remove(&toggle_path);
                }
            },
        })
        .handle()
    }
}

/// Whether `path` has anything in it, without collecting it. A row
/// only needs to know there is something to fold.
fn has_entries(path: &Path) -> bool {
    fs::read_dir(path)
        .is_ok_and(|mut entries| entries.next().is_some())
}

/// `path`'s direct entries, directories before files.
fn build_children(ui: &mut BevyUi, path: &Path) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };

    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            dirs.push(entry_path);
        } else {
            files.push(entry_path);
        }
    }
    dirs.sort();
    files.sort();

    prune_fold_state(ui.world, path, &dirs);

    for dir in dirs {
        ui.compose(FolderRow { path: dir });
    }
    for file in files {
        file_row(ui, &file);
    }
}

/// Drops every open-state key under a direct child of `path` that
/// `current` no longer lists. Bounded to `path`'s own subtree, see
/// [`AssetFoldState`].
fn prune_fold_state(
    world: &mut World,
    path: &Path,
    current: &[PathBuf],
) {
    let mut state = world.resource_mut::<AssetFoldState>();

    // Only the stale roots themselves get cloned here. `filter`
    // runs on borrowed entries before `cloned` ever touches one, so
    // a descendant that isn't actually going anywhere is never
    // copied just to be looked at.
    let stale_roots = state
        .0
        .range((
            Bound::Excluded(path.to_path_buf()),
            Bound::Unbounded,
        ))
        .take_while(|open| open.starts_with(path))
        .filter(|open| {
            open.parent() == Some(path) && !current.contains(open)
        })
        .cloned()
        .collect::<Vec<_>>();

    // Each root's own subtree, not `path`'s whole one. Most of what
    // `path` holds survives untouched and is never cloned at all.
    for root in stale_roots {
        let gone = state
            .0
            .range(root.clone()..)
            .take_while(|open| open.starts_with(&root))
            .cloned()
            .collect::<Vec<_>>();
        for open in gone {
            state.0.remove(&open);
        }
    }
}

/// One file. A recognized asset type (see `AssetKinds`) can be
/// dragged onto an inspector's `Handle<T>` field, in the theme's
/// accent with a grab cursor to say so. Anything else shows dimmer,
/// to read as inert.
fn file_row(ui: &mut BevyUi, path: &Path) {
    let name = display_name(path);
    let label = name.clone();
    let kind = ui.world.resource::<AssetKinds>().kind_of(path);
    let color = if kind.is_some() {
        ui.theme.palette.purple
    } else {
        ui.theme.text_muted
    };

    let mut row = ui.elem(elem!(
        Frame,
        width = percent(100),
        padding = UiRect::vertical(px(3))
    ));
    row.with(move |ui| {
        ui.elem(elem!(
            Label,
            text = name,
            color = Some(color),
            wrap = false
        ));
    });

    if let Some(kind) = kind {
        row.insert(EntityCursor::System(SystemCursorIcon::Grab));
        draggable(&mut row, path.to_path_buf(), kind, label);
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

//! What the editor is built out of.
//!
//! A widget is a struct rather than a `bsn!` scene: its fields are
//! the data, its `#[elem(patch = ...)]` writers and
//! `#[element(build = ...)]` hook say what they mean to bevy, and a
//! binding names one of them, so a value can change without the node
//! being rebuilt.

pub mod dock;

mod button;
mod divider;
mod dropdown;
mod field;
mod frame;
mod icon;
mod inspector;
mod label;
mod overlay;
mod panel;
mod patch;
mod playhead;
mod scroll_area;
mod segmented_control;
mod tab;
mod time_label;
mod time_tick;
mod timeline_action;
mod timeline_block;
mod timeline_gap;
mod timeline_track;

// The cursor traits come too: a binding names a field by walking to
// it, and the walk is what those provide.
pub use button::{
    Button, ButtonCursor, GhostButton, MenuButton, SegmentButton,
    TintButton,
};
pub use divider::{Divider, DividerCursor};
pub use dropdown::{
    Dropdown, DropdownCursor, DropdownItem, DropdownItemCursor,
    DropdownList, DropdownListCursor, DropdownMenu,
};
pub use field::{
    CheckBox, CheckBoxCursor, NumberField, NumberFieldCursor,
    TextField, TextFieldCursor,
};
pub use frame::{Frame, FrameCursor};
pub use icon::{Icon, IconCursor};
// Composers rather than elements, so no cursor or field type: what
// they are handed picks a subtree rather than naming a value, and
// nothing about them is stored to be patched later.
pub use inspector::{
    ComponentInspector, EntityInspector, ResourceInspector,
    display_name,
};
pub use label::{Label, LabelCursor};
pub use overlay::{Overlay, OverlayCursor};
pub use panel::{Panel, PanelCursor};
pub use playhead::{PlayheadLine, PlayheadLineCursor};
pub use scroll_area::{ScrollArea, ScrollAreaCursor};
pub use segmented_control::SegmentedControl;
pub use tab::{
    Tab, TabBar, TabBarCursor, TabCursor, TabRow, TabRowCursor,
};
pub use timeline_action::{TimelineAction, TimelineActionCursor};
pub use timeline_block::{TimelineBlock, TimelineBlockCursor};
pub use timeline_gap::{TimelineGap, TimelineGapCursor};
pub use timeline_track::{TimelineTrack, TimelineTrackCursor};

pub use time_tick::{TimeTick, TimeTickCursor};

pub use time_label::{TimeLabel, TimeLabelCursor};

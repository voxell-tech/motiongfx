//! What the editor is built out of.
//!
//! A widget is a struct rather than a `bsn!` scene: its fields are
//! the data, [`ElementVisual`](fynix_mock::element::ElementVisual)
//! says what they mean to bevy, and a binding names one of them, so
//! a value can change without the node being rebuilt.

pub mod dock;

mod button;
mod divider;
mod field;
mod frame;
mod icon;
mod label;
mod overlay;
mod panel;
mod playhead;
mod tab;
mod timeline_block;
mod timeline_track;

// The cursor traits come too: a binding names a field by walking to
// it, and the walk is what those provide.
pub use button::{
    Button, GhostButton, RawButton, RawButtonCursor, RawButtonField,
    TintButton,
};
pub use divider::{Divider, DividerCursor, DividerField};
pub use field::{
    CheckBox, CheckBoxCursor, CheckBoxField, NumberField,
    NumberFieldCursor, NumberFieldField,
};
pub use frame::{Frame, FrameCursor, FrameField};
pub use icon::{Icon, IconCursor, IconField};
pub use label::{Label, LabelCursor, LabelField};
pub use overlay::{Overlay, OverlayCursor, OverlayField};
pub use panel::{Panel, PanelCursor, PanelField};
pub use playhead::{
    PlayheadLine, PlayheadLineCursor, PlayheadLineField,
};
pub use tab::{
    Tab, TabBar, TabBarCursor, TabBarField, TabCursor, TabField,
    TabRow, TabRowCursor, TabRowField,
};
pub use timeline_block::{
    TimelineBlock, TimelineBlockCursor, TimelineBlockField,
};
pub use timeline_track::{
    TimelineTrack, TimelineTrackCursor, TimelineTrackField,
};

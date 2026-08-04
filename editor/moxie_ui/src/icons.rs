//! Icon asset paths moxie_ui's own widgets reach for, relative to
//! the shared `editor/assets` folder (see `AssetPlugin::file_path` in
//! the consuming app). An app's *own* icons (panel tabs, playback,
//! ...) belong in the app's crate instead — this is only for icons
//! the dock/inspector engine itself draws.

/// Shown when a tab has no [`DockWindowDescriptor::icon`](
/// crate::widgets::dock::DockWindowDescriptor::icon): the slot stays
/// reserved but fully transparent rather than being conditionally
/// omitted.
pub const PLACEHOLDER: &str = "icons/general/placeholder.png";

/// The tab bar's "add tab" button.
pub const PLUS: &str = "icons/general/plus.png";

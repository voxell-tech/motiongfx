//! Widgets: trees built with [`moxie_ui_kernel`] — a `watch`/`bind`
//! composition of [`crate::elements`], not a single static [`Scene`](bevy::prelude::Scene).

pub mod dock;
mod glass_backdrop;
pub mod inspector;

pub use glass_backdrop::{GlassBackdrop, bind_backdrop};

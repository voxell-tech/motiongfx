//! A mock of the fynix element model.

#![no_std]

extern crate alloc;

// Lets the derive emit `::fynix_mock::...` everywhere, including here.
extern crate self as fynix_mock;

pub mod element;
pub mod host;
pub mod kernel;
pub mod lenz;
pub mod store;
pub mod ui;

/// Writes the `Default` an element starts from, before a style and then
/// a call site have had their say.
///
/// Nothing about it is UI: it applies to any struct whose fields want
/// a default other than their own.
pub use fynix_mock_macros::OverrideDefault;

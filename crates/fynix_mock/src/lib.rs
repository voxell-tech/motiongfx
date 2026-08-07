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

//! The `bootmgr-rs` library crate.
//!
//! This is used mainly to expose features such as the parser, boot actions, etc. for external applications such as
//! the integration tests, as well as the fuzzers.

#![cfg_attr(not(any(fuzzing, test, doctest)), no_std)]

/// The primary result type that wraps around [`crate::error::BootError`].
pub type BootResult<T> = Result<T, crate::error::BootError>;

pub mod app;
pub mod boot;
pub mod config;
pub mod error;
pub mod features;
pub mod system;
pub mod ui;

#[cfg(feature = "editor")]
pub mod editor;

extern crate alloc;

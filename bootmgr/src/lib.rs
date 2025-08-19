// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! The `bootmgr-rs` library crate.
//!
//! This is used mainly to expose features such as the parser, boot actions, etc. for external applications such as
//! the integration tests, as well as the fuzzers.
//!
//! This "library" aspect of it also allows for external frontends to be provided beyond that of ratatui, like GUI
//! frontends or an even more minimal user interface.
//!
//! An example basic frontend of this bootloader core can be found in [bootmgr-rs-minimal](https://github.com/some100/bootmgr-rs/tree/main/bootmgr-rs-minimal).
//!
//! ## MSRV
//!
//! The minimum supported rust version is 1.88.0.

#![cfg_attr(not(any(fuzzing, test, doctest)), no_std)]

/// The primary result type that wraps around [`crate::error::BootError`].
pub type BootResult<T> = Result<T, crate::error::BootError>;

pub mod boot;
pub mod config;
pub mod error;
pub mod system;

mod features;

extern crate alloc;

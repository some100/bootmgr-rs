//! The `bootmgr-rs` library crate.
//!
//! This is used mainly to expose features such as the parser, boot actions, etc. for external applications such as
//! the integration tests, as well as the fuzzers.
//!
//! This "library" aspect of it also allows for external frontends to be provided beyond that of ratatui, like GUI
//! frontends or an even more minimal user interface.
//!
//! # Example
//!
//! ```no_run
//! #![no_main]
//! #![no_std]
//!
//! use bootmgr_rs_core::{boot::BootMgr, error::BootError, system::log_backend::UefiLogger};
//! use uefi::{prelude::*, println, proto::console::text::{Input, Key}, system::with_stdout};
//!
//! fn main_func() -> anyhow::Result<Handle> {
//!     uefi::helpers::init().map_err(BootError::Uefi)?;
//!     with_stdout(|f| f.clear())?;
//!     log::set_logger(UefiLogger::static_new())
//!         .map(|()| log::set_max_level(log::LevelFilter::Warn))
//!         .unwrap();
//!
//!     let mut boot_mgr = BootMgr::new()?;
//!
//!     for (i, config) in boot_mgr.list().into_iter().enumerate() {
//!         println!("{i}: {}", config.title.unwrap_or(config.filename));
//!     }
//!     println!("Enter the preferred boot option here:");
//!
//!     let handle = boot::get_handle_for_protocol::<Input>()?;
//!     let mut input = boot::open_protocol_exclusive::<Input>(handle)?;
//!     
//!     let mut events = [ input.wait_for_key_event().unwrap() ];
//!     loop {
//!         boot::wait_for_event(&mut events)?;
//!
//!         match input.read_key()? {
//!             Some(Key::Printable(key)) => {
//!                 let key = char::from(key);
//!                 if let Some(key) = key.to_digit(10) {
//!                     if key < boot_mgr.configs.len() as u32 {
//!                         return Ok(boot_mgr.load(key as usize)?);
//!                     }
//!                 }
//!             }
//!             _ => (),
//!         }
//!     }
//! }
//!
//! #[entry]
//! fn main() -> Status {
//!     let image = main_func().unwrap_or_else(|e| panic!("Error: {e}"));
//!     boot::start_image(image).status()
//! }
//! ```

#![cfg_attr(not(any(fuzzing, test, doctest)), no_std)]

/// The primary result type that wraps around [`crate::error::BootError`].
pub type BootResult<T> = Result<T, crate::error::BootError>;

pub mod boot;
pub mod config;
pub mod error;
pub mod system;

extern crate alloc;

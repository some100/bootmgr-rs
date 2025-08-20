// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! A GUI frontend for `bootmgr`.
//!
//! This is a user interface that aims to show how (comparatively) simple it is
//! to use a GUI frontend or a frontend of any kind with `bootmgr-rs`.

#![no_main]
#![no_std]

extern crate alloc;

use bootmgr::boot::action::reboot;
use thiserror::Error;
use uefi::{Handle, ResultExt, Status, boot::start_image, entry};

use crate::app::App;

mod app;
mod editor;
mod input;
mod ui;

/// An error that may occur when running the application.
#[derive(Error, Debug)]
pub enum MainError {
    /// An error occurred with the boot manager.
    #[error("Boot Error: {0}")]
    BootError(#[from] bootmgr::error::BootError),
    /// A fatal error occurred while running the Slint UI.
    #[error("Slint Error: {0}")]
    SlintError(slint::PlatformError),
    /// The input was closed for some reason.
    #[error("Input protocol was closed")]
    InputClosed,
}

/// The actual main function of the program.
///
/// # Errors
///
/// May return an `Error` if a failure occurs while the app is running.
fn main_func() -> Result<Option<Handle>, MainError> {
    // This is all done to ensure that GOP, Input, etc. are properly dropped before the next program is started.
    // If the image was simply booted directly from the tryboot function, then it would result in this program
    // still holding on to GOP and other protocols, which in the case of loading this program again, would result
    // in a panic.
    let app = App::new()?;
    app.run()
}

/// The main function of the program.
///
/// This will not panic on fatal error, since the error message will have already been displayed in a popup.
#[entry]
fn main() -> Status {
    match main_func() {
        Ok(Some(image)) => start_image(image).status(),
        Err(_) => reboot::reset(),
        _ => Status::SUCCESS,
    }
}

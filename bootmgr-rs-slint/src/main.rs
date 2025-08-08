//! A GUI frontend for `bootmgr-rs-core`.
#![no_main]
#![no_std]

extern crate alloc;

use bootmgr_rs_core::error::BootError;
use thiserror::Error;
use uefi::{Handle, ResultExt, Status, boot::start_image, entry};

use crate::app::App;

mod app;
mod ui;

/// An error that may occur when running the application.
#[derive(Error, Debug)]
pub enum MainError {
    /// An error occurred with the boot manager.
    #[error("Boot Error")]
    BootError(#[from] bootmgr_rs_core::error::BootError),
    /// A fatal error occurred while running the Slint UI.
    #[error("Slint Error")]
    SlintError(slint::PlatformError),
}

fn main_func() -> Result<Option<Handle>, MainError> {
    uefi::helpers::init().map_err(BootError::Uefi)?;

    // This is all done to ensure that GOP, Input, etc. are properly dropped before the next program is started.
    // If the image was simply booted directly from the tryboot function, then it would result in this program
    // still holding on to GOP and other protocols, which in the case of loading this program again, would result
    // in a panic.
    let mut app = App::new()?;
    app.run()
}

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error occurred while running: {e}")); // panic on critical error
    match image {
        Some(image) => start_image(image).status(),
        None => Status::SUCCESS,
    }
}

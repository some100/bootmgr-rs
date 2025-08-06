//! A GUI frontend for `bootmgr-rs-core`.
#![no_main]
#![no_std]

extern crate alloc;

use core::cell::RefCell;

use alloc::rc::Rc;
use bootmgr_rs_core::error::BootError;
use slint::ComponentHandle;
use thiserror::Error;
use uefi::{Status, entry};

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

fn main_func() -> Result<(), MainError> {
    uefi::helpers::init().map_err(BootError::Uefi)?;
    let app = Rc::new(RefCell::new(App::new()?));

    let ui = app.borrow_mut().get_ui()?;

    ui.on_tryboot(move |x| app.borrow_mut().try_boot(x));
    ui.run().map_err(MainError::SlintError)?;

    Ok(())
}

#[entry]
fn main() -> Status {
    main_func().unwrap_or_else(|e| panic!("Error occurred while running: {e}")); // panic on critical error
    Status::SUCCESS
}

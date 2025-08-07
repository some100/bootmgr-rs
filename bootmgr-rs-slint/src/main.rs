//! A GUI frontend for `bootmgr-rs-core`.
#![no_main]
#![no_std]

extern crate alloc;

use core::cell::{Cell, RefCell};

use alloc::rc::Rc;
use bootmgr_rs_core::error::BootError;
use slint::ComponentHandle;
use thiserror::Error;
use uefi::{boot::start_image, entry, Handle, ResultExt, Status};

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
    let app = Rc::new(RefCell::new(App::new()?)); // yeah....
    let boot_mgr = app.borrow().boot_mgr.clone();

    let image = Rc::new(Cell::new(None));
    let image_weak = Rc::downgrade(&image);

    #[allow(clippy::cast_sign_loss)]
    app.borrow_mut().ui.on_tryboot(move |x| {
        if let Some(image) = image_weak.upgrade() 
        {
            image.set(boot_mgr.borrow_mut().load(x as usize).ok());
            let _ = slint::quit_event_loop();
        }
    });
    app.borrow().ui.run().map_err(MainError::SlintError)?;

    Ok(image.take())
}

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error occurred while running: {e}")); // panic on critical error
    match image {
        Some(image) => start_image(image).status(),
        None => Status::SUCCESS,
    }
}

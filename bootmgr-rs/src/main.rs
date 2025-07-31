//! The `bootmgr-rs` application.
//!
//! This provides a [`main`] function that runs the library.
//! Because UEFI applications may only return Status, every failable call here will panic on error.

#![no_main]
#![no_std]

use bootmgr_rs::{
    app::App, error::BootError, system::log_backend::UefiLogger, ui::ratatui_backend::UefiBackend,
};

use ratatui_core::terminal::Terminal;
use thiserror::Error;
use uefi::{boot::start_image, prelude::*};

/// An error that may occur when running the application.
#[derive(Error, Debug)]
enum MainError {
    /// An error occurred with the boot manager.
    #[error("Boot Error")]
    BootError(#[from] bootmgr_rs::error::BootError),
    /// This should not happen.
    #[error("Set logger was already previously called")]
    SetLogger(log::SetLoggerError),
}

fn main_func() -> Result<Option<Handle>, MainError> {
    uefi::helpers::init().map_err(BootError::Uefi)?;
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .map_err(MainError::SetLogger)?;
    let backend = UefiBackend::new()?;
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new()?;

    match app.run(&mut terminal)? {
        Some(image) => {
            app.close(terminal);
            Ok(Some(image))
        }
        None => Ok(None),
    }
}

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error occurred while running: {e}")); // panic on critical error
    match image {
        Some(image) => start_image(image).status(),
        None => Status::SUCCESS, // this means the program was exited naturally
    }
}

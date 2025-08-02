//! The `bootmgr-rs` application.
//!
//! This provides a [`main`] function that runs the library.
//! It uses every single part of the library, and the final executable (on release builds) is a little bit over 300 KB
//! with all features.
//! Because UEFI applications may only return Status, every failable call here will panic on error.

#![no_main]
#![no_std]

extern crate alloc;

use bootmgr_rs_core::{error::BootError, system::log_backend::UefiLogger};

use ratatui_core::terminal::Terminal;
use thiserror::Error;
use uefi::{boot::start_image, prelude::*};

use crate::{app::App, ui::ratatui_backend::UefiBackend};

mod app;
mod features;
mod ui;

#[cfg(feature = "editor")]
mod editor;

/// An error that may occur when running the application.
#[derive(Error, Debug)]
pub enum MainError {
    /// An error occurred with the boot manager.
    #[error("Boot Error")]
    BootError(#[from] bootmgr_rs_core::error::BootError),
    /// This should not happen. Set logger is called once at the start of the program.
    #[error("Set logger was already previously called")]
    SetLogger(log::SetLoggerError),
    /// An error occurred while running the App.
    #[error("App Error")]
    AppError(#[from] crate::app::AppError),
}

fn main_func() -> Result<Option<Handle>, MainError> {
    uefi::helpers::init().map_err(BootError::Uefi)?;
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Warn))
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

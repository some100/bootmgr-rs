// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! The `bootmgr-rs` application.
//!
//! This provides a [`main`] function that runs the library.
//! It uses every single part of the library, and the final executable (on release builds) is a little bit over 300 KB
//! with all features.
//! Because UEFI applications may only return Status, every failable call here will panic on error.

#![no_main]
#![no_std]

extern crate alloc;

use bootmgr::system::log_backend::UefiLogger;

use ratatui_core::terminal::Terminal;
use thiserror::Error;
use uefi::{boot::start_image, prelude::*};

use crate::{app::App, ui::ratatui_backend::UefiBackend};

mod app;
mod features;
mod ui;

#[cfg(feature = "editor")]
mod editor;

/// The global logging instance.
static LOGGER: UefiLogger = UefiLogger::new();

/// An error that may occur when running the application.
#[derive(Error, Debug)]
pub enum MainError {
    /// An error occurred with the boot manager.
    #[error("Boot Error: {0}")]
    BootError(#[from] bootmgr::error::BootError),
    /// An error occurred while running the App.
    #[error("App Error: {0}")]
    AppError(#[from] crate::app::AppError),
}

/// The actual main function of the program.
///
/// # Errors
///
/// May return an `Error` if the terminal backend could not be initialized, or a failure occurs while the `App` is initalized
/// or ran.
fn main_func() -> Result<Option<Handle>, MainError> {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Warn)); // if the logger was already set, then ignore it

    let backend = UefiBackend::new()?;
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new()?;

    let image = app.run(&mut terminal)?;

    image.map_or(Ok(None), |image| Ok(Some(image)))
}

/// The main function of the program
///
/// # Panics
///
/// This will panic if the `main_func` returns an error.
#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error occurred while running: {e}")); // panic on critical error
    image.map_or(Status::SUCCESS, |image| start_image(image).status())
}

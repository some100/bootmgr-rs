#![no_main]
#![no_std]

mod app;
mod boot;
mod error;
mod parsers;
mod system;
mod ui;

extern crate alloc;

use crate::{app::App, system::log_backend::UefiLogger, ui::ratatui_backend::UefiBackend};

use ratatui_core::terminal::Terminal;
use uefi::{boot::start_image, prelude::*};

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Somehow, the UEFI helpers failed to initialize");
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .unwrap();
    let backend = UefiBackend::new().expect("Failed to initialize backend");
    let mut terminal = Terminal::new(backend).expect("Failed to initialize Terminal");
    let mut app = App::new().expect("Failed to create App");

    if let Some(image) = app
        .run(&mut terminal)
        .expect("Error occurred while running App")
    {
        drop(terminal);
        drop(app);
        start_image(image).status()
    } else {
        Status::SUCCESS
    }
}

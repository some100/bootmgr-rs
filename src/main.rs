#![no_main]
#![no_std]
#![warn(clippy::mod_module_files)]

use bootmgr_rs::{app::App, system::log_backend::UefiLogger, ui::ratatui_backend::UefiBackend};

use ratatui_core::terminal::Terminal;
use uefi::{boot::start_image, prelude::*};

#[entry]
fn main() -> Status {
    uefi::helpers::init().expect("Failed to initialize UEFI helpers");
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to set logger");
    let backend = UefiBackend::new().expect("Failed to initialize UEFI terminal backend");
    let mut terminal = Terminal::new(backend).expect("Failed to crate Terminal");
    let mut app = App::new().expect("Error occurred while initializing App");

    if let Some(image) = app
        .run(&mut terminal)
        .expect("Error occurred while running App")
    {
        app.close(terminal);
        start_image(image).status()
    } else {
        Status::SUCCESS
    }
}

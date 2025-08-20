// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Extremely minimal skeleton for a boot loader.
//!
//! This mostly serves as an example minimal frontend for the bootmgr crate, and is probably more similar
//! to ELILO than anything systemd-boot or bootmgr.
//! This should probably not be used over the reference application using ratatui if possible, unless for some
//! reason you wanted a UI even more minimal.

#![no_main]
#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use bootmgr::{
    boot::{BootMgr, action::reboot},
    error::BootError,
    system::{helper::locate_protocol, log_backend::UefiLogger},
};
use log::error;
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key, Output},
    system::with_stdout,
};

/// The global logging instance.
static LOGGER: UefiLogger = UefiLogger::new();

/// The actual main function of the program, which returns a [`Result`].
///
/// `Box<dyn core::error::Error>` is used here mainly for simplicity purposes (we simply will propagate all these errors).
///
/// # Errors
///
/// May return an `Error` if an error occurs while the boot manager is created, or there is no input protocol, or an error
/// occurred while loading an image.
fn main_func() -> Result<Handle, Box<dyn core::error::Error>> {
    uefi::helpers::init().map_err(BootError::Uefi)?; // initialize helpers (for print)
    with_stdout(Output::clear)?;
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Warn));

    let mut boot_mgr = BootMgr::new()?;

    for (i, config) in boot_mgr.list().iter().enumerate() {
        println!("{i}: {}", config.get_preferred_title(Some(i))); // print every boot option present
    }
    println!("Enter the preferred boot option here:");

    let mut input = locate_protocol::<Input>()?;

    let mut events = [input
        .wait_for_key_event()
        .ok_or("Failed to get key event from input")?];
    loop {
        boot::wait_for_event(&mut events)?; // wait for a key press

        if let Some(Key::Printable(key)) = input.read_key()? {
            let key = char::from(key);
            if let Some(key) = key.to_digit(10)
                && (key as usize) < boot_mgr.list().len()
            {
                return Ok(boot_mgr.load(key as usize)?); // load the boot option
            }
        }
    }
}

/// The main function of the program.
///
/// # Panics
///
/// Will return a panic if an error occurs while `main_func` is ran.
#[entry]
fn main() -> Status {
    match main_func() {
        Ok(image) => boot::start_image(image).status(),
        Err(e) => {
            error!("Fatal error occurred: {e}");
            error!("Automatically restarting in 10 seconds");

            // simple restart timer is used here for simplicity
            boot::stall(10_000_000);
            reboot::reset();
        }
    }
}

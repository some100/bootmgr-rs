//! Extremely minimal skeleton for a boot loader.
//!
//! This mostly serves as an example minimal frontend for the bootmgr-rs-core crate, and is probably more similar
//! to ELILO than anything systemd-boot or bootmgr.
//! This should probably not be used over the reference application using ratatui if possible, unless for some
//! reason you wanted a UI even more minimal.

#![no_main]
#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use bootmgr_rs_core::{boot::BootMgr, error::BootError, system::log_backend::UefiLogger};
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key, Output},
    system::with_stdout,
};

/// The actual main function of the program, which returns a [`Result`].
///
/// `Box<dyn core::error::Error>` is used here mainly for simplicity purposes (we simply will propagate all these errors).
fn main_func() -> Result<Handle, Box<dyn core::error::Error>> {
    uefi::helpers::init().map_err(BootError::Uefi)?; // initialize helpers (for print)
    with_stdout(Output::clear)?;
    let _ = log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Warn));

    let mut boot_mgr = BootMgr::new()?;

    for (i, config) in boot_mgr.list().iter().enumerate() {
        println!("{i}: {}", config.get_preferred_title(Some(i))); // print every boot option present
    }
    println!("Enter the preferred boot option here:");

    let handle = boot::get_handle_for_protocol::<Input>()?;
    let mut input = boot::open_protocol_exclusive::<Input>(handle)?;

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

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error: {e}")); // panic on critical error
    boot::start_image(image).status() // finally start the image
}

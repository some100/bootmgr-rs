//! Extremely minimal skeleton for a boot loader.
//!
//! This mostly serves as an example minimal frontend for the bootmgr-rs-core crate, and is probably more similar
//! to ELILO than anything systemd-boot or bootmgr.
//! This should probably not be used over the reference application using ratatui if possible, unless for some
//! reason you wanted a UI even more minimal.

#![no_main]
#![no_std]

use anyhow::anyhow;
use bootmgr_rs_core::{boot::BootMgr, error::BootError, system::log_backend::UefiLogger};
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key, Output},
    system::with_stdout,
};

/// The actual main function of the program, which returns an [`anyhow::Result`].
fn main_func() -> anyhow::Result<Handle> {
    uefi::helpers::init().map_err(BootError::Uefi)?; // initialize helpers (for print)
    with_stdout(Output::clear)?; // clear output in case this was loaded from shell, or is loading itself
    let _ = log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Warn));

    let mut boot_mgr = BootMgr::new()?; // create the main boot manager

    for (i, config) in boot_mgr.list().iter().enumerate() {
        println!("{i}: {}", config.title.as_ref().unwrap_or(&config.filename)); // print every boot option present
    }
    println!("Enter the preferred boot option here:");

    let handle = boot::get_handle_for_protocol::<Input>()?; // get a random input
    let mut input = boot::open_protocol_exclusive::<Input>(handle)?; // get input protocol

    // create an event for waiting for a key press
    let mut events = [input
        .wait_for_key_event()
        .ok_or(anyhow!("Failed to get key event from input"))?];
    loop {
        boot::wait_for_event(&mut events)?; // wait for a key press

        if let Some(Key::Printable(key)) = input.read_key()? {
            let key = char::from(key);
            if let Some(key) = key.to_digit(10) // convert the key to a number
                && (key as usize) < boot_mgr.list().len()
            // check if the number is less than the length of hte list
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

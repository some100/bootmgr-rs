//! Extremely basic skeletion for a boot loader.
//!
//! This mostly serves as an example basic frontend for the bootmgr-rs-core crate, and is probably more similar
//! to ELILO than anything systemd-boot or bootmgr.
//! This should probably not be used over the reference application using ratatui if possible, unless for some
//! reason you wanted a UI even more basic.

#![no_main]
#![no_std]

use bootmgr_rs_core::{boot::BootMgr, error::BootError, system::log_backend::UefiLogger};
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key, Output},
    system::with_stdout,
};

/// The actual main function of the program, which returns an [`anyhow::Result`].
fn main_func() -> anyhow::Result<Handle> {
    uefi::helpers::init().map_err(BootError::Uefi)?;
    with_stdout(Output::clear)?;
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Warn))
        .unwrap();

    let mut boot_mgr = BootMgr::new()?;

    for (i, config) in boot_mgr.list().into_iter().enumerate() {
        println!("{i}: {}", config.title.unwrap_or(config.filename));
    }
    println!("Enter the preferred boot option here:");

    let handle = boot::get_handle_for_protocol::<Input>()?;
    let mut input = boot::open_protocol_exclusive::<Input>(handle)?;

    let mut events = [input.wait_for_key_event().unwrap()];
    loop {
        boot::wait_for_event(&mut events)?;

        if let Some(Key::Printable(key)) = input.read_key()? {
            let key = char::from(key);
            if let Some(key) = key.to_digit(10)
                && (key as usize) < boot_mgr.configs.len()
            {
                return Ok(boot_mgr.load(key as usize)?);
            }
        }
    }
}

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error: {e}"));
    boot::start_image(image).status()
}

# bootmgr-rs-core

A framework for creating boot managers in Rust. Has support for Windows, BLS, and UKIs, with auto detection for the fallback bootloader, UEFI shell, and macOS.

# Example
```rust
#![no_main]
#![no_std]

use bootmgr_rs_core::{boot::BootMgr, error::BootError, system::log_backend::UefiLogger};
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key, Output},
    system::with_stdout,
};

fn main_func() -> anyhow::Result<Handle> {
    uefi::helpers::init().map_err(BootError::Uefi)?;
    with_stdout(Output::clear)?; // clear screen first before we do anything
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Warn))
        .unwrap(); // Initialize logger for errors from core crate to show up

    let mut boot_mgr = BootMgr::new()?;

    for (i, config) in boot_mgr.list().into_iter().enumerate() {
        println!("{i}: {}", config.title.unwrap_or(config.filename)); // get all boot entries in system
    }
    println!("Enter the preferred boot option here:");

    // get input
    let handle = boot::get_handle_for_protocol::<Input>()?;
    let mut input = boot::open_protocol_exclusive::<Input>(handle)?;

    let mut events = [input.wait_for_key_event().unwrap()];
    loop {
        boot::wait_for_event(&mut events)?;

        if let Some(Key::Printable(key)) = input.read_key()? {
            let key = char::from(key);
            if let Some(key) = key.to_digit(10)
                && (key as usize) < boot_mgr.configs.len()
            { // test if input as number falls in range, and load that entry to get an image Handle
                return Ok(boot_mgr.load(key as usize)?);
            }
        }
    }
}

#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| panic!("Error: {e}")); // get an image Handle or panic on error
    boot::start_image(image).status() // start the image
}
```

This example can also be found in [`bootmgr-rs-minimal`](https://github.com/some100/bootmgr-rs/tree/main/bootmgr-rs-minimal).
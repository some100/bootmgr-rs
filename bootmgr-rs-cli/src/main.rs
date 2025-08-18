// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! A command line interface frontend to `bootmgr-rs`.

#![no_main]
#![no_std]

extern crate alloc;

use alloc::string::ToString;

use bootmgr_rs_core::{BootResult, boot::BootMgr, system::log_backend::UefiLogger};
use getargs::{Arg, Options};
use uefi::{
    Handle, ResultExt, Status, boot, cstr16, entry, println, proto::loaded_image::LoadedImage,
};

/// The global logging instance.
static LOGGER: UefiLogger = UefiLogger::new();

/// The actual main function of the program, which returns an [`anyhow::Result`].
///
/// # Errors
///
/// May return an `Error` if the program could not obtain the `LoadedImage` protocol.
fn main_func() -> BootResult<Option<Handle>> {
    uefi::helpers::init()?; // initialize helpers (for print)

    let load_options = {
        let handle = boot::image_handle();
        let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;
        loaded_image
            .load_options_as_cstr16()
            .unwrap_or(cstr16!("bootmgr-rs-cli.efi")) // there is at least one argument, which is the filename
            .to_string()
    }; // loaded_image dropped here

    let mut options = load_options.split_whitespace();

    let Some(app_filename) = options.next() else {
        println!("Error: No load options were passed to the program");
        return Ok(None);
    };

    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Warn));

    let mut boot_mgr = BootMgr::new()?;

    let mut opts = Options::new(options);
    while let Ok(Some(arg)) = opts.next_arg() {
        match arg {
            Arg::Short('l') | Arg::Long("list") => {
                for (i, config) in boot_mgr.list().iter().enumerate() {
                    println!(
                        "{i}: {} ({})",
                        config.get_preferred_title(Some(i)),
                        config.filename,
                    );
                }
                return Ok(None);
            }
            Arg::Short('b') | Arg::Long("boot") => {
                let Ok(value) = opts.value() else {
                    println!("Error: An index was not passed into the boot argument");
                    return Ok(None);
                };
                let idx = match value.parse() {
                    Ok(idx) => idx,
                    Err(e) => {
                        println!(
                            "Error: {e} (The value passed to the boot argument could not be parsed as a number)"
                        );
                        return Ok(None);
                    }
                };
                if idx >= boot_mgr.list().len() {
                    println!(
                        "Error: The value passed to the boot argument was not in range of the list"
                    );
                    return Ok(None);
                }

                return Ok(Some(boot_mgr.load(idx)?));
            }
            Arg::Short('h') | Arg::Long("help") => break, // ignore any other arguments and break out of the while loop when help is specified
            Arg::Short(invalid) => println!("Error: Unknown short argument: -{invalid}"),
            Arg::Long(invalid) => println!("Error: Unknown long argument: --{invalid}"),
            Arg::Positional(invalid) => println!("Error: Unknown positional argument: {invalid}"),
        }
    }

    println!(
        r"Usage: {app_filename} [OPTIONS] [ARGS]...
                    
-h, --help       display this help and exit
-l, --list       display boot options and exit
-b, --boot       boot the given boot option index
"
    );

    Ok(None)
}

/// The main function of the program.
///
/// This will not panic on a fatal error, rather, it will return control to the UEFI shell (or the firmware menu).
/// This program is intended to be ran as a shell script, so panicking here would not make any sense.
#[entry]
fn main() -> Status {
    let image = main_func().unwrap_or_else(|e| {
        println!("Error: {e}");
        None // dont panic when an error occurs, instead, just exit.
    });
    image.map_or(Status::SUCCESS, |image| boot::start_image(image).status())
}

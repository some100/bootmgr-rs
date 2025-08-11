#![no_main]
#![no_std]

// Integration tests for bootmgr-rs.

// DISCLAIMER: This code extensively uses unwrap and expect, as any errors in testing should be treated as fatal.

use anyhow::anyhow;
use bootmgr_rs_core::{
    boot::action::reboot,
    system::{helper::locate_protocol, log_backend::UefiLogger},
};
use uefi::{
    prelude::*,
    println,
    proto::console::text::{Input, Key},
};

use crate::{
    action::test_custom_actions,
    fs::test_filesystem,
    load::{check_loaded, test_loading},
    variables::{check_variable, test_variables},
};

mod action;
mod fs;
mod load;
mod variables;

fn main_func() -> anyhow::Result<()> {
    uefi::helpers::init()?;
    let _ = log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info));

    check_loaded()?;
    check_variable()?;

    println!("Select the test you would like to do:");
    println!("1. Custom action test");
    println!("2. Variables test");
    println!("3. Filesystem test");
    println!("4. Load image test");
    println!(
        "It's recommended that the tests are tested in order, as they will rely on each other in that order."
    );
    loop {
        if let Key::Printable(char) = read_key()? {
            let char = char::from(char);
            return match char {
                '1' => test_custom_actions(),
                '2' => Ok(test_variables()?),
                '3' => Ok(test_filesystem()?),
                '4' => test_loading(),
                _ => Ok(()),
            };
        }
    }
}

#[entry]
fn main() -> Status {
    main_func().unwrap_or_else(|e| panic!("Failed to run test: {e}"));
    Status::SUCCESS
}

fn press_for_reboot() -> ! {
    let _ = read_key();
    reboot::reset();
}

fn read_key() -> anyhow::Result<Key> {
    let mut input = locate_protocol::<Input>()?;
    let key_event = input
        .wait_for_key_event()
        .ok_or_else(|| anyhow!("Input device not present"))?;
    let mut events = [key_event];
    boot::wait_for_event(&mut events).discard_errdata()?;
    let key = input.read_key()?;
    key.ok_or_else(|| anyhow!("Input device not present"))
}

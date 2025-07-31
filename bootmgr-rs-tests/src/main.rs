#![no_main]
#![no_std]

// Integration tests for bootmgr-rs.

// DISCLAIMER: This code extensively uses unwrap and expect, as any errors in testing should be treated as fatal.

use bootmgr_rs::{BootResult, boot::action::reboot, system::log_backend::UefiLogger};
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

fn main_func() -> BootResult<()> {
    uefi::helpers::init()?;
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to set logger"); // set up logger so that errors produced by the library will get caught as well

    check_loaded();
    check_variable();

    println!("Select the test you would like to do:");
    println!("1. Custom action test");
    println!("2. Variables test");
    println!("3. Filesystem test");
    println!("4. Load image test");
    println!(
        "It's recommended that the tests are tested in order, as they will rely on each other in that order."
    );
    loop {
        if let Key::Printable(char) = read_key() {
            let char = char::from(char);
            return match char {
                '1' => test_custom_actions(),
                '2' => test_variables(),
                '3' => test_filesystem(),
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

fn read_key() -> Key {
    let handle = boot::get_handle_for_protocol::<Input>().unwrap();
    let mut input = boot::open_protocol_exclusive::<Input>(handle).unwrap();
    let mut events = [input.wait_for_key_event().unwrap()];
    boot::wait_for_event(&mut events).unwrap();
    input.read_key().unwrap().unwrap()
}

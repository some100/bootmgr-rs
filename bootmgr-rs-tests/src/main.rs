#![no_main]
#![no_std]

// Integration tests for bootmgr-rs.

use bootmgr_rs::system::log_backend::UefiLogger;
use uefi::{prelude::*, println, proto::console::text::{Input, Key}};

use crate::{fs::test_filesystem, load::test_loading, test_custom_action::test_custom_actions, variables::{check_variable, test_variables}};

mod fs;
mod load;
mod test_custom_action;
mod variables;

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    log::set_logger(UefiLogger::static_new())
        .map(|()| log::set_max_level(log::LevelFilter::Info))
        .expect("Failed to set logger"); // set up logger so that errors produced by the library will get caught as well

    check_variable();

    println!("Select the test you would like to do:");
    println!("1. Custom action test");
    println!("2. Variables test");
    println!("3. Filesystem test");
    println!("4. Load image test");
    loop {
        match read_key() {
            Key::Printable(char) => {
                let char = char::from(char);
                match char {
                    '1' => test_custom_actions(),
                    '2' => test_variables(),
                    '3' => test_filesystem(),
                    '4' => test_loading(),
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

fn read_key() -> Key {
    let handle = boot::get_handle_for_protocol::<Input>().unwrap();
    let mut input = boot::open_protocol_exclusive::<Input>(handle).unwrap();
    let mut events = [ input.wait_for_key_event().unwrap() ];
    boot::wait_for_event(&mut events).unwrap();
    input.read_key().unwrap().unwrap()
}

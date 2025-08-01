use bootmgr_rs::boot::action::*;
use uefi::{println, proto::console::text::Key};

use crate::read_key;

pub fn test_custom_actions() -> anyhow::Result<()> {
    println!("Select the boot option that you want to test:");
    println!("1. Reboot");
    println!("2. Shutdown");
    println!("3. Reboot to Firmware Setup");
    loop {
        if let Key::Printable(char) = read_key()? {
            let char = char::from(char);
            match char {
                '1' => reboot::reset(),
                '2' => shutdown::shutdown(),
                '3' => firmware::reset_to_firmware(),
                _ => (),
            }
            if matches!(char, '1' | '2' | '3') {
                unreachable!();
            }
        }
    }
}

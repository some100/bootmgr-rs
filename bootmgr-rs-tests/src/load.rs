use bootmgr_rs::{boot::{action::reboot, loader::load_boot_option}, config::builder::ConfigBuilder, system::fs::check_file_exists};
use uefi::{boot, cstr16, println, proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem}, CStr16};

use crate::read_key;

const SHELL_PATH: &CStr16 = cstr16!("\\shellx64.efi");
const FALLBACK_PATH: &CStr16 = cstr16!("\\EFI\\BOOT\\BOOTx64.efi");

pub fn test_loading() {
    println!("Will try to load an image from either {SHELL_PATH} or {FALLBACK_PATH} on same filesystem");
    println!("Press a key to continue");
    let _ = read_key();

    let efi = {
        let mut fs = boot::get_image_file_system(boot::image_handle()).unwrap();
        if check_file_exists(&mut fs, SHELL_PATH) {
            SHELL_PATH
        } else if check_file_exists(&mut fs, FALLBACK_PATH) {
            FALLBACK_PATH
        } else {
            println!("Cannot test if load image works, as {SHELL_PATH} and {FALLBACK_PATH} do not exist");
            println!("Press a key to reboot");
            let _ = read_key();
            reboot::reset();
        }
    }; // fs dropped here

    let handle = {
        let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
        let device_handle = loaded_image.device().unwrap();
        let device_path = boot::open_protocol_exclusive::<DevicePath>(device_handle).unwrap();
        boot::locate_device_path::<SimpleFileSystem>(&mut &*device_path).unwrap()
    }; // so that the handle will be able to be opened for loading the boot option

    let config = ConfigBuilder::new("", "")
        .efi(efi)
        .handle(handle)
        .build();

    let handle = load_boot_option(&config).unwrap();
    boot::start_image(handle).unwrap();
}
use bootmgr_rs::{
    BootResult,
    boot::loader::load_boot_option,
    config::builder::ConfigBuilder,
    system::{
        fs::check_file_exists,
        variable::{get_variable, set_variable},
    },
};
use uefi::{
    CStr16, boot, cstr16, println,
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem},
};

use crate::{press_for_reboot, read_key};

const LOADED_VARIABLE_NAME: &CStr16 = cstr16!("LoadedFromPrevTest");
const SHELL_PATH: &CStr16 = cstr16!("\\shellx64.efi");
const FALLBACK_PATH: &CStr16 = cstr16!("\\EFI\\BOOT\\BOOTx64.efi");

pub fn check_loaded() {
    if let Ok(num) = get_variable::<usize>(LOADED_VARIABLE_NAME, None)
        && num != 0
    {
        set_variable::<usize>(LOADED_VARIABLE_NAME, None, None, None).unwrap();
        println!("Successfully passed load image test");
        println!(
            "If a panic from unwrap resulted before this, then the test was not actually passed."
        );
        println!("Press a key to reboot");
        press_for_reboot();
    }
}

pub fn test_loading() -> BootResult<()> {
    println!(
        "Will try to load an image from either {SHELL_PATH} or {FALLBACK_PATH} on same filesystem"
    );
    println!("Press a key to continue");
    let _ = read_key();

    let efi = {
        let mut fs = boot::get_image_file_system(boot::image_handle())?;
        if check_file_exists(&mut fs, SHELL_PATH) {
            SHELL_PATH
        } else if check_file_exists(&mut fs, FALLBACK_PATH) {
            FALLBACK_PATH
        } else {
            println!(
                "Cannot test if load image works, as {SHELL_PATH} and {FALLBACK_PATH} do not exist"
            );
            println!("Press a key to reboot");
            press_for_reboot();
        }
    }; // fs dropped here

    let handle = {
        let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle())?;
        let device_handle = loaded_image
            .device()
            .unwrap_or_else(|| panic!("Image handle was not loaded from a storage device"));
        let device_path = boot::open_protocol_exclusive::<DevicePath>(device_handle)?;
        boot::locate_device_path::<SimpleFileSystem>(&mut &*device_path)?
    }; // so that the handle will be able to be opened for loading the boot option

    let config = ConfigBuilder::new("", "").efi(efi).handle(handle).build();

    let handle = load_boot_option(&config)?;
    set_variable::<usize>(LOADED_VARIABLE_NAME, None, None, Some(1))?;
    boot::start_image(handle)?;

    Ok(())
}

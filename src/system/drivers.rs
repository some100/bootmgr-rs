//! Loads drivers located in \EFI\BOOT\drivers, or some other path configured in `BootConfig`
//!
//! This will also check if the drivers are actual drivers and not just random EFI executables. If they are not drivers,
//! then the `load_driver` function will error. It may also reconnect all handles so that the recently loaded drivers
//! may take effect.

use alloc::{string::String, vec::Vec};
use log::error;
use thiserror::Error;
use uefi::{
    CStr16, boot,
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::file::FileInfo},
};

use crate::{
    BootResult,
    system::{
        fs::read_filtered_dir,
        helper::{get_device_path, get_path_cstr, str_to_cstr},
    },
};

/// An `Error` that may result from loading drivers.
#[derive(Error, Debug)]
pub enum DriverError {
    /// An EFI file is not a supported driver type
    #[error("Unsupported EFI file: {0}")]
    Unsupported(String),
}

// Loads a driver from a given [`FileInfo`], then starts the driver using StartImage
fn load_driver(driver_path: &CStr16, file: &FileInfo, vec: &mut Vec<u8>) -> BootResult<()> {
    let handle_path = boot::open_protocol_exclusive::<DevicePath>(boot::image_handle())?;
    let path_cstr = get_path_cstr(driver_path, file.file_name())?;

    let path = get_device_path(&handle_path, &path_cstr, vec)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };
    let handle = boot::load_image(boot::image_handle(), src)?;

    let image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;

    if image.code_type() != boot::MemoryType::BOOT_SERVICES_CODE
        && image.code_type() != boot::MemoryType::RUNTIME_SERVICES_CODE
    {
        return Err(DriverError::Unsupported(file.file_name().into()).into());
    }

    Ok(boot::start_image(handle)?)
}

/// Loads every driver from the same filesystem that the bootloader was loaded from.
///
/// # Errors
///
/// May return an `Error` if either the image handle doesn't support `SimpleFileSystem` or
/// there are literally no handles present on the system, both of which are quite unlikely
pub fn load_drivers(driver_path: &str) -> BootResult<()> {
    let driver_path = str_to_cstr(driver_path)?;
    let mut fs = boot::get_image_file_system(boot::image_handle())?;
    let dir = read_filtered_dir(&mut fs, &driver_path, ".efi");

    let mut vec = Vec::new();
    let mut driver_loaded = false;

    // dir will be an alphanumeric sorted directory. if any drivers have dependencies on another drivers,
    // it should be named such that it will be loaded after that driver.
    for file in dir {
        load_driver(&driver_path, &file, &mut vec)?;

        driver_loaded = true;
    }
    if driver_loaded {
        reconnect_drivers()?; // only reconnect drivers when a driver was loaded
    }
    Ok(())
}

// Reconnects every handle so that drivers can take effect
fn reconnect_drivers() -> BootResult<()> {
    let handles = boot::locate_handle_buffer(boot::SearchType::AllHandles)?;
    for handle in handles.iter() {
        let _ = boot::connect_controller(*handle, None, None, true);
    }
    Ok(())
}

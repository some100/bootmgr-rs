// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Loads drivers located in \EFI\BOOT\drivers, or some other path configured in `BootConfig`
//!
//! This will also check if the drivers are actual drivers and not just random EFI executables. If they are not drivers,
//! then the `load_driver` function will error. It may also reconnect all handles so that the recently loaded drivers
//! may take effect.

use alloc::string::String;

use log::error;
use thiserror::Error;
use uefi::{
    CStr16, boot,
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::file::FileInfo},
};

use crate::{
    BootResult,
    boot::secure_boot::shim::shim_load_image,
    system::{
        fs::UefiFileSystem,
        helper::{get_path_cstr, join_to_device_path, str_to_cstr},
    },
};

/// An `Error` that may result from loading drivers.
#[derive(Error, Debug)]
pub enum DriverError {
    /// An EFI file is not a supported driver type
    #[error("Unsupported EFI file: \"{0}\"")]
    Unsupported(String),
}

/// Loads a driver from a given [`FileInfo`], then starts the driver using `StartImage`
///
/// # Errors
///
/// May return an `Error` if the image handle does not support [`DevicePath`], or the driver (image) could not be
/// loaded, or the image is not a valid driver, or the image could not be started.
fn load_driver(driver_path: &CStr16, file: &FileInfo, buf: &mut [u8]) -> BootResult<()> {
    let handle_path = boot::open_protocol_exclusive::<DevicePath>(boot::image_handle())?;
    let path_cstr = get_path_cstr(driver_path, file.file_name())?;

    let path = join_to_device_path(&handle_path, &path_cstr, buf)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };

    // use Shim if available to load the image, incase the driver is in mok or something
    let handle = shim_load_image(boot::image_handle(), src)?;

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
pub(crate) fn load_drivers(driver_path: &str) -> BootResult<()> {
    let driver_path = str_to_cstr(driver_path)?;
    let mut fs = UefiFileSystem::from_image_fs()?;

    let dir = fs.read_filtered_dir(&driver_path, ".efi");

    // it should be rare for a devicepath to be greater than 2048 bytes. this is a generous amount that should cover
    // for most cases
    let mut buf = [0; 2048];
    let mut driver_loaded = false;

    // dir will be an alphanumeric sorted directory. if any drivers have dependencies on another drivers,
    // it should be named such that it will be loaded after that driver.
    for file in dir {
        if let Err(e) = load_driver(&driver_path, &file, &mut buf) {
            error!("Failed to load driver {}: {e}", file.file_name());
        } else {
            driver_loaded = true;
        }
    }
    if driver_loaded {
        reconnect_drivers()?; // only reconnect drivers when a driver was loaded
    }
    Ok(())
}

/// Reconnects every handle so that drivers can take effect
///
/// # Errors
///
/// May return an `Error` if there is literally no handle on the system, of literally any kind.
fn reconnect_drivers() -> BootResult<()> {
    let handles = boot::locate_handle_buffer(boot::SearchType::AllHandles)?;
    for handle in handles.iter() {
        let _ = boot::connect_controller(*handle, None, None, true);
    }
    Ok(())
}

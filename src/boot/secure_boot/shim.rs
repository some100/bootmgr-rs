//! Shim integration into secure boot.
//!
//! For Shim versions earlier than v16, this will allow the usage of the Shim validator in order
//! to check if an image is valid semi-independently of the firmware.

use core::ptr::NonNull;

use uefi::{
    Handle,
    boot::{self, ScopedProtocol},
    proto::{device_path::DevicePath, media::fs::SimpleFileSystem, shim::ShimLock},
};

use crate::{
    BootResult,
    boot::secure_boot::{
        SecureBootError, install_security_override, secure_boot_enabled,
        uninstall_security_override,
    },
    system::{fs::read, helper::device_path_to_text, protos::ShimImageLoader},
};

fn validate_from_device_path(
    mut device_path: &DevicePath,
    shim: &mut ScopedProtocol<ShimLock>,
) -> BootResult<()> {
    let handle = boot::locate_device_path::<SimpleFileSystem>(&mut device_path)?;
    let mut fs = boot::open_protocol_exclusive::<SimpleFileSystem>(handle)?;

    let path = device_path_to_text(device_path)?;
    let file_buffer = read(&mut fs, &path)?;

    Ok(shim.verify(&file_buffer)?)
}

fn shim_loaded() -> bool {
    boot::get_handle_for_protocol::<ShimLock>().is_ok()
}

// Checks if shim is recent enough to hook onto LoadImage and not require custom security override
fn shim_is_recent() -> bool {
    boot::get_handle_for_protocol::<ShimImageLoader>().is_ok()
}

fn shim_validate(
    _ctx: Option<NonNull<u8>>,
    device_path: Option<&DevicePath>,
    file_buffer: Option<&mut [u8]>,
    _file_size: usize,
) -> BootResult<()> {
    let handle = boot::get_handle_for_protocol::<ShimLock>()?;
    let mut shim = boot::open_protocol_exclusive::<ShimLock>(handle)?;

    if let Some(file_buffer) = file_buffer {
        return Ok(shim.verify(file_buffer)?);
    }

    if let Some(device_path) = device_path {
        return validate_from_device_path(device_path, &mut shim);
    }

    Err(SecureBootError::NoDevicePathOrFile.into())
}

/// Loads an image, optionally verifying it with Shim if it exists.
///
/// `LoadImage` uses the `SecurityArch` or `Security2Arch` protocols when loading an image and secure boot is enabled.
/// Due to this, we can temporarily override these protocols with our own custom hooks, then uninstall them once we're finished
/// loading the image. Even if we aren't using Shim, we can still benefit from Secure Boot as `LoadImage` will automatically
/// validate those images without our input. This is even if we don't install those security overrides.
///
/// # Errors
///
/// May return an `Error` if the [`boot::load_image`] fails.
pub fn shim_load_image(parent: Handle, source: boot::LoadImageSource<'_>) -> BootResult<Handle> {
    if !shim_loaded() || shim_is_recent() || !secure_boot_enabled() {
        return Ok(boot::load_image(parent, source)?);
    }

    install_security_override(shim_validate, None);

    let handle = boot::load_image(parent, source);

    uninstall_security_override();

    Ok(handle?)
}

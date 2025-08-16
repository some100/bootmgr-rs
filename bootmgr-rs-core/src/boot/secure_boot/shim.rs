// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Shim integration into secure boot.
//!
//! The main export of this module is `shim_load_image`, which will optionally verify the image with Shim if required.
//! To explain the function briefly, if Shim is old enough (lower than version 16) and is present, it will install a custom
//! `SecurityOverrideGuard` to replace the firmware validator with a custom validator using [`ShimLock`] to verify images.
//!
//! If Shim v16+ is loaded (indicated using [`ShimImageLoader`]), then the Shim validator is already installed and we can simply
//! do nothing.
//!
//! The same is done for if Shim is not present or secure boot is disabled.

use core::ptr::NonNull;

use uefi::{
    Handle, Identify,
    boot::{self, ScopedProtocol},
    cstr16,
    proto::{device_path::DevicePath, media::fs::SimpleFileSystem, shim::ShimLock},
    runtime::VariableVendor,
};

use crate::{
    BootResult,
    boot::secure_boot::{SecureBootError, SecurityOverrideGuard, secure_boot_enabled},
    system::{
        fs::UefiFileSystem,
        helper::{device_path_to_text, locate_protocol},
        protos::ShimImageLoader,
        variable::{get_variable, set_variable},
    },
};

/// Checks an image using [`ShimLock`] protocol when provided the [`DevicePath`].
///
/// # Errors
///
/// May return an `Error` if the device path does not lead to a handle supporting [`SimpleFileSystem`],
/// or the system does not support `DevicePathToText`, or the file does not exist in the filesystem.
fn validate_from_device_path(
    mut device_path: &DevicePath,
    shim: &ScopedProtocol<ShimLock>,
) -> BootResult<()> {
    let handle = boot::locate_device_path::<SimpleFileSystem>(&mut device_path)?;
    let mut fs = UefiFileSystem::from_handle(handle)?;

    let path = device_path_to_text(device_path)?;
    let file_buffer = fs.read(&path)?;

    Ok(shim.verify(&file_buffer)?)
}

/// Checks for the presence of [`ShimLock`].
fn shim_loaded() -> bool {
    boot::get_handle_for_protocol::<ShimLock>().is_ok()
}

/// Checks if shim is recent enough to hook onto `LoadImage` and not require custom security override
///
/// It does this by checking for presence of [`ShimImageLoader`], which is Shim v16+ only. If
/// [`ShimImageLoader`] is loaded, that indicates that shim had already replaced the function pointers
/// with its own validators, so there would be nothing for us to do.
fn shim_is_recent() -> bool {
    boot::get_handle_for_protocol::<ShimImageLoader>().is_ok()
}

/// Shim validator with [`super::Validator`] function signature.
fn shim_validate(
    _ctx: Option<NonNull<u8>>,
    device_path: Option<&DevicePath>,
    file_buffer: Option<&mut [u8]>,
    _file_size: usize,
) -> BootResult<()> {
    let shim = locate_protocol::<ShimLock>()?;

    if let Some(file_buffer) = file_buffer {
        return Ok(shim.verify(file_buffer)?);
    }

    if let Some(device_path) = device_path {
        return validate_from_device_path(device_path, &shim);
    }

    Err(SecureBootError::NoDevicePathOrFile.into())
}

/// Ask Shim to keep its protocol around, in case we need to verify more images (for example, after loading drivers with Shim)
fn shim_retain_protocol() -> BootResult<()> {
    let vendor = VariableVendor(ShimLock::GUID);
    if !matches!(
        get_variable::<bool>(cstr16!("ShimRetainProtocol"), Some(vendor)),
        Ok(true)
    ) {
        set_variable::<bool>(
            cstr16!("ShimRetainProtocol"),
            Some(vendor),
            None,
            Some(true),
        )?;
    }
    Ok(())
}

/// Loads an image, optionally verifying it with Shim if it exists.
///
/// `LoadImage` uses the `SecurityArch` or `Security2Arch` protocols when loading an image and secure boot is enabled.
/// Due to this, we can temporarily override these protocols with our own custom hooks, then uninstall them once we're finished
/// loading the image. Even if we aren't using Shim, we can still benefit from Secure Boot as `LoadImage` will automatically
/// validate those images without our input. This is even if we don't install those security overrides.
///
/// When Shim is not loaded, or Shim v16+ is used, or Secure Boot is not enabled, this function simply attempts to load an image
/// without any prior security override, then return the handle from that. Installing a security override is not required for Shim
/// v16+ as [`ShimImageLoader`] is used, which hooks onto `LoadImage` and friends and automatically does the security overrides for us.
///
/// # Errors
///
/// May return an `Error` if the [`boot::load_image`] fails.
pub(crate) fn shim_load_image(
    parent: Handle,
    source: boot::LoadImageSource<'_>,
) -> BootResult<Handle> {
    if !shim_loaded() || shim_is_recent() || !secure_boot_enabled() {
        return Ok(boot::load_image(parent, source)?);
    }

    shim_retain_protocol()?;

    let _guard = SecurityOverrideGuard::new(shim_validate, None);

    let handle = boot::load_image(parent, source);

    Ok(handle?)
} // override dropped (uninstalled) here

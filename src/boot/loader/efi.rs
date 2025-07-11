#![allow(clippy::cast_possible_truncation)]
//! The boot loader for EFI executables

use crate::{
    boot::{action::handle_action, devicetree::install_devicetree},
    config::Config,
    error::BootError,
    system::helper::get_device_path,
};

use alloc::{boxed::Box, vec::Vec};
use uefi::{
    CStr16, CString16, Handle,
    boot::{self, ScopedProtocol, image_handle},
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem},
};

/// Loads a boot option from a given [`Config`].
///
/// This function loads an EFI executable defined in config.efi, and optionally
/// may also install devicetree for ARM devices, and can set load options in
/// config.options.
///
/// May return an `Error` for many reasons, see [`uefi::boot::load_image`] and [`uefi::boot::open_protocol_exclusive`]
pub fn load_boot_option(config: &Config) -> Result<Handle, BootError> {
    handle_action(&config.action)?;

    let handle = config
        .handle
        .ok_or(BootError::ConfigMissingHandle(config.filename.clone()))?;
    let mut fs = boot::open_protocol_exclusive(handle)?;

    let file = config.efi.clone();

    let s = CString16::try_from(&*file)?;

    let handle = load_image_from_path(handle, &s)?;

    setup_image(&mut fs, handle, config)
}

fn load_image_from_path(handle: Handle, path: &CStr16) -> Result<Handle, BootError> {
    let dev_path = boot::open_protocol_exclusive::<DevicePath>(handle)?;
    let mut vec = Vec::new();
    let path = get_device_path(&dev_path, path, &mut vec)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };
    Ok(boot::load_image(image_handle(), src)?)
}

// Sets up the image for boot with load options and devicetree.
fn setup_image(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
    config: &Config,
) -> Result<Handle, BootError> {
    if let Some(devicetree) = &config.devicetree {
        install_devicetree(devicetree, fs)?;
    }

    let options = config.options.as_ref().map_or("", |v| v);
    let mut image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;
    let load_options = Box::new(CString16::try_from(options)?);
    let load_options_size = load_options.num_bytes() as u32;

    // the load options must last for until the image is started. the easiest way to do this is simply to leak the Box so it becomes static
    // this is necessary so that the load options can last beyond this functions lifetime.
    let load_options_ptr: &'static CStr16 = Box::leak(load_options);

    unsafe {
        // SAFETY: this is safe since we already leaked load_options
        image.set_load_options(load_options_ptr.as_ptr().cast(), load_options_size);
    }
    Ok(handle)
}

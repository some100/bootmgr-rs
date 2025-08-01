//! The boot loader for EFI executables
//!
//! This will also handle devicetree installs and Shim authentication if either are available.

use core::cell::RefCell;

use crate::{
    BootResult,
    boot::{
        devicetree::install_devicetree,
        loader::{LoadError, get_efi},
        secure_boot::shim::shim_load_image,
    },
    config::Config,
    system::helper::{get_device_path, str_to_cstr},
};

use uefi::{
    CStr16, CString16, Handle,
    boot::{self, ScopedProtocol},
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem},
};

/// An instance of `LoadOptions` that remains for the lifetime of the program.
/// This is because load options must last long enough so that it can be safely
/// passed into [`LoadOptions::set_load_options`].
static LOAD_OPTIONS: LoadOptions = LoadOptions {
    options: RefCell::new(None),
};

/// Storage struct for a [`CString16`] with load options.
struct LoadOptions {
    /// [`RefCell`] wrapper around the load options.
    options: RefCell<Option<CString16>>,
}

impl LoadOptions {
    /// Set the current load options from a [`CStr16`] slice.
    fn set(&self, s: &CStr16) {
        let mut options = self.options.borrow_mut();
        *options = Some(s.into());
    }

    /// Get the current load options as a possibly null u8 raw pointer.
    fn get(&self) -> Option<*const u8> {
        self.options
            .borrow()
            .as_ref()
            .map(|x| x.as_ptr().cast::<u8>())
    }

    /// Get the number of bytes of the load options.
    fn size(&self) -> usize {
        self.options.borrow().as_ref().map_or(0, |x| x.num_bytes())
    }

    /// Set the load options of an image to the load options of the struct.
    fn set_load_options(&self, image: &mut ScopedProtocol<LoadedImage>) {
        if let Some(ptr) = self.get() {
            // it is quite unlikely that the load options will literally exceed 4 gb in length, so its safe to truncate
            let size = match u32::try_from(self.size()) {
                Ok(size) => size,
                _ => u32::MAX,
            };
            unsafe {
                // SAFETY: this should ONLY be used with a static cell, as the pointer must last long enough for the loaded image to use it
                image.set_load_options(ptr, size);
            }
        }
    }
}

// SAFETY: uefi is a single threaded environment, thread safety is irrelevant
unsafe impl Sync for LoadOptions {}

/// Loads a boot option from a given [`Config`] through EFI.
///
/// This function loads an EFI executable defined in config.efi, and optionally
/// may also install devicetree for ARM devices, and can set load options in
/// config.options.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`boot::load_image`] and [`boot::open_protocol_exclusive`]
pub fn load_boot_option(config: &Config) -> BootResult<Handle> {
    let handle = *config
        .handle
        .ok_or_else(|| LoadError::ConfigMissingHandle(config.filename.clone()))?;

    let mut fs = boot::open_protocol_exclusive(handle)?;

    let file = get_efi(config)?;

    let s = str_to_cstr(file)?;

    let handle = load_image_from_path(handle, &s)?;

    setup_image(&mut fs, handle, config)
}

/// Load an image given a [`Handle`] and a path.
///
/// # Errors
///
/// May return an `Error` if the handle does not support [`DevicePath`], or the image could not be loaded.
fn load_image_from_path(handle: Handle, path: &CStr16) -> BootResult<Handle> {
    let dev_path = boot::open_protocol_exclusive::<DevicePath>(handle)?;
    let mut buf = [0; 2048]; // it should be rare for a devicepath to exceed 2048 bytes
    let path = get_device_path(&dev_path, path, &mut buf)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::BootSelection,
    };
    shim_load_image(boot::image_handle(), src) // this will either load with shim validation, or just load the image
}

/// Sets up the image for boot with load options and optionally loading a devicetree.
///
/// # Errors
///
/// May return an `Error` if the image does not support [`LoadedImage`], or, if a devicetree
/// is present, the devicetree could not be installed.
fn setup_image(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
    config: &Config,
) -> BootResult<Handle> {
    let load_options = &LOAD_OPTIONS;

    if let Some(devicetree) = &config.devicetree {
        install_devicetree(devicetree, fs)?;
    }

    let options = config.options.as_deref().unwrap_or_default();
    let mut image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;

    load_options.set(&str_to_cstr(options)?);

    load_options.set_load_options(&mut image);

    Ok(handle)
}

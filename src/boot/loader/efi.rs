//! The boot loader for EFI executables
//!
//! This will also handle devicetree installs and Shim authentication if either are available.

use core::cell::UnsafeCell;

use crate::{
    BootResult,
    boot::{devicetree::install_devicetree, loader::LoadError, secure_boot::shim::shim_load_image},
    config::Config,
    system::helper::{get_device_path, str_to_cstr},
};

use alloc::vec::Vec;
use uefi::{
    CStr16, CString16, Handle,
    boot::{self, ScopedProtocol, image_handle},
    proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem},
};

// An instance of LoadOptions that remains for the lifetime of the program.
// This is because load options must last long enough so that it can be safely
// passed into set_load_options.
static LOAD_OPTIONS: LoadOptions = LoadOptions {
    options: UnsafeCell::new(None),
};

struct LoadOptions {
    options: UnsafeCell<Option<CString16>>,
}

impl LoadOptions {
    fn set(&self, s: &CStr16) {
        let options = unsafe { &mut *self.options.get() };
        *options = Some(s.into());
    }

    fn get(&self) -> Option<*const u8> {
        unsafe {
            (*self.options.get())
                .as_ref()
                .map(|x| x.as_ptr().cast::<u8>())
        }
    }

    fn size(&self) -> usize {
        unsafe { (*self.options.get()).as_ref() }.map_or(0, |x| x.num_bytes())
    }

    fn set_load_options(&self, image: &mut ScopedProtocol<LoadedImage>) {
        if let Some(ptr) = self.get() {
            // it is quite unlikely that the load options will literally exceed 4 gb in length, so its safe to truncate
            let size = match u32::try_from(self.size()) {
                Ok(size) => size,
                _ => u32::MAX,
            };
            unsafe {
                image.set_load_options(ptr, size);
            }
        }
    }

    fn clear(&self) {
        let options = unsafe { &mut *self.options.get() };
        *options = None;
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

    let file = config
        .efi
        .as_deref()
        .ok_or_else(|| LoadError::ConfigMissingEfi(config.filename.clone()))?;

    let s = str_to_cstr(file)?;

    let handle = load_image_from_path(handle, &s)?;

    setup_image(&mut fs, handle, config)
}

fn load_image_from_path(handle: Handle, path: &CStr16) -> BootResult<Handle> {
    let dev_path = boot::open_protocol_exclusive::<DevicePath>(handle)?;
    let mut vec = Vec::new();
    let path = get_device_path(&dev_path, path, &mut vec)?;

    let src = boot::LoadImageSource::FromDevicePath {
        device_path: &path,
        boot_policy: uefi::proto::BootPolicy::ExactMatch,
    };
    shim_load_image(image_handle(), src)
}

// Sets up the image for boot with load options and devicetree.
fn setup_image(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
    config: &Config,
) -> BootResult<Handle> {
    let load_options = &LOAD_OPTIONS;

    config
        .devicetree
        .as_ref()
        .map(|x| install_devicetree(x, fs))
        .transpose()?;

    let options = config.options.as_deref().unwrap_or_default();
    let mut image = boot::open_protocol_exclusive::<LoadedImage>(handle)?;

    load_options.set(&str_to_cstr(options)?);

    load_options.set_load_options(&mut image);

    // now that we have already set the load options in the image, we clear it since we do not need it anymore
    load_options.clear();

    Ok(handle)
}

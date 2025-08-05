//! Boot loading re-exports
//!
//! This mainly provides the function [`load_boot_option`], which will redirect [`Config`]s to the respective boot loaders
//! depending on the action set. It is essentially a wrapper around running the `run()` method on the [`Config`]'s action field.

use alloc::string::String;

use thiserror::Error;
use uefi::Handle;

use crate::{BootResult, config::Config};

pub mod efi;
pub mod tftp;

/// An `Error` that may result from loading an image.
#[derive(Error, Debug)]
pub enum LoadError {
    /// A [`Config`] did not have a [`Handle`] when required.
    #[error("Config \"{0}\" attempted to boot without a handle")]
    ConfigMissingHandle(String),

    /// A [`Config`] did not have an EFI defined when required.
    #[error("Config \"{0}\" attempted to boot without an EFI executable")]
    ConfigMissingEfi(String),

    /// Failed to parse a string as an IP address.
    #[error("Failed to parse as IP address")]
    IpParse(#[from] core::net::AddrParseError),

    /// The HTTP response did not have a valid content-length header.
    #[error("Nonexistent or invalid content length header found in address \"{0}\"")]
    InvalidContentLen(String),
}

/// Loads a boot option given a [`Config`].
///
/// It simply delegates to [`super::action::BootAction::run`].
///
/// # Errors
///
/// May return an `Error` if any of the actions fail.
///
/// # Example
///
/// ```no_run
/// // this example starts the fallback boot loader on the same partition as the image handle.
///
/// use bootmgr_rs_core::{boot::loader::load_boot_option, config::builder::ConfigBuilder};
/// use uefi::{
///     boot,
///     proto::{
///         device_path::DevicePath,
///         loaded_image::LoadedImage,
///         media::fs::SimpleFileSystem
///     }
/// };
///
/// let handle = {
///     let loaded_image =
///         boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
///     let device_handle = loaded_image.device().expect("Image was not loaded from a filesystem");
///     let device_path = boot::open_protocol_exclusive::<DevicePath>(device_handle).unwrap();
///     boot::locate_device_path::<SimpleFileSystem>(&mut &*device_path).unwrap()
/// }; // so that the handle will be able to be opened for loading the boot option
///
/// let config = ConfigBuilder::new("foo.bar", ".bar").efi_path("/efi/boot/bootx64.efi").fs_handle(handle).build();
///
/// let image = load_boot_option(&config).unwrap();
///
/// boot::start_image(image).unwrap();
/// ```
pub fn load_boot_option(config: &Config) -> BootResult<Handle> {
    config.action.run(config)
}

/// Get an EFI path from a [`Config`].
///
/// # Errors
///
/// May return an `Error` if the [`Config`] does not contain an EFI path.
fn get_efi(config: &Config) -> Result<&String, LoadError> {
    config
        .efi_path
        .as_deref()
        .ok_or_else(|| LoadError::ConfigMissingEfi(config.filename.clone()))
}

#[cfg(test)]
mod tests {
    use crate::{boot::action::BootAction, error::BootError};

    use super::*;

    #[test]
    fn test_missing_handle() {
        let config = Config {
            fs_handle: None,
            action: BootAction::BootEfi,
            ..Default::default()
        };
        assert!(matches!(
            load_boot_option(&config),
            Err(BootError::LoadError(LoadError::ConfigMissingHandle(_)))
        ));
    }
}

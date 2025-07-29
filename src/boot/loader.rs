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
    #[error("Config {0} attempted to boot without a handle")]
    ConfigMissingHandle(String),

    /// A [`Config`] did not have an EFI defined when required.
    #[error("Config {0} attempted to boot without an EFI executable")]
    ConfigMissingEfi(String),

    /// Failed to parse a string as an IP address.
    #[error("Failed to parse as IP address")]
    IpParse(#[from] core::net::AddrParseError),

    /// The HTTP response did not have a valid content-length header.
    #[error("Nonexistent or invalid content length header found in address {0}")]
    InvalidContentLen(String),
}

/// Loads a boot option given a [`Config`].
///
/// It simply delegates to [`super::action::BootAction::run`].
///
/// # Errors
///
/// May return an `Error` if any of the actions fail.
pub fn load_boot_option(config: &Config) -> BootResult<Handle> {
    config.action.run(config)
}

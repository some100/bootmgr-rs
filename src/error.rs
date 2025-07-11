//! Provides [`BootError`], which encapsulates other errors

use alloc::string::String;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootError {
    /// An error with UEFI, or a service from the [`uefi`] crate.
    #[error("UEFI Error")]
    Uefi(#[from] uefi::Error),

    /// A [`String`] could not be converted into a [`uefi::data_types::CString16`].
    #[error("String Conversion Error")]
    FromStr(#[from] uefi::data_types::FromStrError),

    /// A Device Path could not be built. This probably should not happen.
    #[error("Device Path Build Error")]
    Build(#[from] uefi::proto::device_path::build::BuildError),

    /// The Device Path could not be appended to an existing one for some reason.
    #[error("Device Path Utilities Error")]
    DevPathUtil(#[from] uefi::proto::device_path::DevicePathUtilitiesError),

    /// This should not happen.
    #[error("set_logger was already called")]
    SetLogger(log::SetLoggerError),

    /// The [`crate::config::Config`] was missing a [`uefi::Handle`].
    #[error("Config {0} missing handle")]
    ConfigMissingHandle(String),

    /// The Input protocol was closed for any reason.
    #[error("Keyboard input protocol is closed")]
    InputClosed,

    /// The file was unsupported for loading a driver.
    #[error("File {0} is unsupported")]
    Unsupported(String),

    /// The path is not valid.
    #[error("Config {0} contains invalid executable path {1}")]
    InvalidPath(String, String),

    /// The [`crate::config::Config`] does not match the system architecture.
    #[error("Config {0} has non-matching architecture")]
    NonMatchingArch(String),

    /// The specified executable does not exist at the path.
    #[error("{0} executable does not exist at path {1}")]
    NotExist(&'static str, String),

    /// The EFI executable was not a valid PE exeuctable.
    #[cfg(feature = "uki")]
    #[error("Error while parsing PE binary: {0}")]
    Pe(pelite::Error),

    /// The BCD could not be parsed for any reason.
    #[cfg(feature = "windows")]
    #[error("Hive Parse Error")]
    Hive(#[from] nt_hive::NtHiveError),

    /// The BCD was missing a required key for parsing.
    #[cfg(feature = "windows")]
    #[error("BCD missing key: {0}")]
    BcdMissingKey(&'static str),

    /// The BCD was missing a required value inside of a key for parsing.
    #[cfg(feature = "windows")]
    #[error("BCD missing Element value in key: {0}")]
    BcdMissingElement(&'static str),
}

// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Provides [`BootError`], which encapsulates other errors

use thiserror::Error;

/// An `Error` resulting from the program.
#[derive(Error, Debug)]
pub enum BootError {
    /// An error with UEFI, or a service from the [`uefi`] crate.
    #[error("UEFI Error: {0}")]
    Uefi(#[from] uefi::Error),

    /// A `String` could not be converted into a `CString`
    #[error("String Conversion Error: {0}")]
    StrError(#[from] crate::system::helper::StrError),

    /// An error occurred while performing filesystem operations.
    #[error("Filesystem Error: {0}")]
    FsError(#[from] crate::system::fs::FsError),

    /// An error occurred while validating an image with Secure Boot.
    #[error("Secure Boot Error: {0}")]
    SecureBootError(#[from] crate::boot::secure_boot::SecureBootError),

    /// An error occurred while building a `DevicePath`.
    #[error("DevicePath Error: {0}")]
    DevicePathError(#[from] crate::system::helper::DevicePathError),

    /// An error occurred while loading an image.
    #[error("Load Image Error: {0}")]
    LoadError(#[from] crate::boot::loader::LoadError),

    /// An error occurred while loading a driver.
    #[error("Load Driver Error: {0}")]
    DriverError(#[from] crate::system::drivers::DriverError),

    /// An error occurred while loading a devicetree.
    #[error("Devicetree Error: {0}")]
    DevicetreeError(#[from] crate::boot::devicetree::DevicetreeError),

    /// An error occurred while interacting with UEFI variables.
    #[error("UEFI Variable Error: {0}")]
    VarError(#[from] crate::system::variable::VarError),

    /// The UKI executable could not be parsed for any reason.
    #[cfg(feature = "uki")]
    #[error("Uki Parse Error: {0}")]
    UkiError(#[from] crate::config::parsers::uki::UkiError),

    /// The BCD could not be parsed for any reason.
    #[cfg(feature = "windows_bcd")]
    #[error("Win Parse Error: {0}")]
    WinError(#[from] crate::config::parsers::windows::windows_bcd::WinError),
}

// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Provides [`BootError`], which encapsulates other errors

use thiserror::Error;

/// An `Error` resulting from the program.
#[derive(Error, Debug)]
pub enum BootError {
    /// An error with UEFI, or a service from the [`uefi`] crate.
    #[error("UEFI Error")]
    Uefi(#[from] uefi::Error),

    /// A `String` could not be converted into a `CString`
    #[error("String Conversion Error")]
    StrError(#[from] crate::system::helper::StrError),

    /// An error occurred while performing filesystem operations.
    #[error("Filesystem Error")]
    FsError(#[from] crate::system::fs::FsError),

    /// An error occurred while validating an image with Secure Boot.
    #[error("Secure Boot Error")]
    SecureBootError(#[from] crate::boot::secure_boot::SecureBootError),

    /// An error occurred while building a `DevicePath`.
    #[error("DevicePath Error")]
    DevicePathError(#[from] crate::system::helper::DevicePathError),

    /// An error occurred while loading an image.
    #[error("Load Image Error")]
    LoadError(#[from] crate::boot::loader::LoadError),

    /// An error occurred while loading a driver.
    #[error("Load Driver Error")]
    DriverError(#[from] crate::system::drivers::DriverError),

    /// An error occurred while loading a devicetree.
    #[error("Devicetree Error")]
    DevicetreeError(#[from] crate::boot::devicetree::DevicetreeError),

    /// The UKI executable could not be parsed for any reason.
    #[cfg(feature = "uki")]
    #[error("Uki Parse Error")]
    UkiError(#[from] crate::config::parsers::uki::UkiError),

    /// The BCD could not be parsed for any reason.
    #[cfg(feature = "windows")]
    #[error("Win Parse Error")]
    WinError(#[from] crate::config::parsers::windows::WinError),
}

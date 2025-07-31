//! `newtype` definitions for fields of [`super::Config`]
//!
//! At the moment, this includes the following type definitions:
//! - [`MachineId`] (constructor enforces strict 32 char length, hexchars only, and will lowercase)
//! - [`SortKey`] (constructor will filter out invalid characters, such as non-ASCII)
//! - [`EfiPath`] and [`DevicetreePath`] (constructor will check if path is a valid UEFI path)
//! - [`Architecture`] (constructor will check if it is a supported architecture)
//! - [`FsHandle`] (constructor will check if the [`Handle`] has support for [`SimpleFileSystem`])

use core::ops::Deref;

use alloc::{borrow::ToOwned, string::String};
use thiserror::Error;
use uefi::{Handle, boot, proto::media::fs::SimpleFileSystem};

use crate::system::{
    fs::check_path_valid,
    helper::{check_machine_id_valid, check_sort_key_valid, normalize_path},
};

/// Errors that may happen from invalid inputs to the respective constructors.
#[derive(Error, Debug)]
pub enum TypeError {
    /// The machine ID was invalid.
    #[error("\"{0}\" is not a valid machine id")]
    MachineId(String),

    /// The sort key was invalid.
    #[error("\"{0}\" is not a valid sort key")]
    SortKey(String),

    /// The path was invalid.
    #[error("\"{0}\" is not a valid path")]
    Path(String),

    /// The architecture was invalid.
    #[error("\"{0}\" is not a valid architecture")]
    Architecture(String),

    /// The handle did not support [`SimpleFileSystem`].
    #[error("\"{0:?}\" is not a valid Handle that supports SimpleFileSystem")]
    Handle(Handle),
}

/// A newtype wrapper around a valid machine ID.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineId(String);

impl MachineId {
    /// Creates a new [`MachineId`].
    ///
    /// # Errors
    ///
    /// May return an `Error` the machine id is not a valid machine id, if it contains invalid
    /// characters or is not exactly 32 chars in length.
    pub fn new(machine_id: &str) -> Result<Self, TypeError> {
        if check_machine_id_valid(machine_id) {
            Ok(Self(machine_id.to_ascii_lowercase()))
        } else {
            Err(TypeError::MachineId(machine_id.to_owned()))
        }
    }
}

impl Deref for MachineId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype wrapper around a valid sort key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SortKey(String);

impl SortKey {
    /// Creates a new [`SortKey`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the sort key is invalid, such as if it contains
    /// invalid characters.
    pub fn new(sort_key: &str) -> Result<Self, TypeError> {
        if check_sort_key_valid(sort_key) {
            Ok(Self(sort_key.to_owned()))
        } else {
            Err(TypeError::SortKey(sort_key.to_owned()))
        }
    }
}

impl Deref for SortKey {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype wrapper around a valid devicetree path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DevicetreePath(String);

impl DevicetreePath {
    /// Creates a new [`DevicetreePath`]. It will also replace any forward slashes with backslashes.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the path is invalid.
    pub fn new(devicetree: &str) -> Result<Self, TypeError> {
        let devicetree = normalize_path(devicetree);
        if check_path_valid(&devicetree) {
            Ok(Self(devicetree))
        } else {
            Err(TypeError::Path(devicetree))
        }
    }
}

impl Deref for DevicetreePath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype wrapper around a valid architecture.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Architecture(String);

impl Architecture {
    /// Creates a new [`Architecture`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the architecture is not supported. It must be one of the
    /// four values: `x86`, `x64`, `arm`, and `aa64`.
    pub fn new(arch: &str) -> Result<Self, TypeError> {
        match arch {
            "x86" | "x64" | "arm" | "aa64" => Ok(Self(arch.to_owned())),
            _ => Err(TypeError::Architecture(arch.to_owned())),
        }
    }
}

impl Deref for Architecture {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype wrapper around a valid EFI path.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EfiPath(String);

impl EfiPath {
    /// Creates a new [`EfiPath`]. It will also replace any forward slashes with backslashes.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the path is invalid.
    pub fn new(efi: &str) -> Result<Self, TypeError> {
        let efi = normalize_path(efi);
        if check_path_valid(&efi) {
            Ok(Self(efi))
        } else {
            Err(TypeError::Path(efi))
        }
    }
}

impl Deref for EfiPath {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A newtype wrapper around a [`Handle`] supporting [`SimpleFileSystem`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FsHandle(Handle);

impl FsHandle {
    /// Creates a new [`FsHandle`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the [`Handle`] does not support [`SimpleFileSystem`]
    pub fn new(handle: Handle) -> Result<Self, TypeError> {
        let params = boot::OpenProtocolParams {
            handle,
            agent: boot::image_handle(),
            controller: None,
        };
        match boot::test_protocol::<SimpleFileSystem>(params) {
            Ok(true) => Ok(Self(handle)),
            _ => Err(TypeError::Handle(handle)),
        }
    }
}

impl Deref for FsHandle {
    type Target = Handle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_key() {
        let sort_key = SortKey::new("a-valid-sort-key");
        assert!(sort_key.is_ok());
        assert_eq!(*sort_key.unwrap(), "a-valid-sort-key".to_owned());
        let sort_key = SortKey::new(";'[];\\[]-=invalid sort key");
        assert!(sort_key.is_err());
    }

    #[test]
    fn test_machine_id() {
        let machine_id = MachineId::new("93274530989549038301177646597349");
        assert!(machine_id.is_ok());
        assert_eq!(
            *machine_id.unwrap(),
            "93274530989549038301177646597349".to_owned()
        );
        let machine_id = MachineId::new("invalidthing");
        assert!(machine_id.is_err());
        let machine_id = MachineId::new("1");
        assert!(machine_id.is_err());
        let machine_id = MachineId::new("1234567890abcdefghijklmnopqrstu");
        assert!(machine_id.is_err());
    }

    #[test]
    fn test_dtb_path() {
        let devicetree = DevicetreePath::new("\\foo\\bar.dtb");
        assert!(devicetree.is_ok());
        assert_eq!(*devicetree.unwrap(), "\\foo\\bar.dtb".to_owned());
        let devicetree = DevicetreePath::new("\\** / : ???? .dtb");
        assert!(devicetree.is_err());
    }

    #[test]
    fn test_arch() {
        let arch = Architecture::new("x64");
        assert!(arch.is_ok());
        assert_eq!(*arch.unwrap(), "x64".to_owned());
        let arch = Architecture::new("notreal64");
        assert!(arch.is_err());
    }

    #[test]
    fn test_efi_path() {
        let efi = EfiPath::new("\\foo\\bar.efi");
        assert!(efi.is_ok());
        assert_eq!(*efi.unwrap(), "\\foo\\bar.efi".to_owned());
        let efi = EfiPath::new(":somethinginvalid*\">><?????;.ef");
        assert!(efi.is_err());
    }
}

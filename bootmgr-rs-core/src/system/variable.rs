// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! UEFI variable storage helpers.
//!
//! These store a value into a UEFI variable in a custom vendor namespace.

use alloc::{vec, vec::Vec};
use uefi::{
    CStr16, Status, guid,
    runtime::{self, VariableAttributes, VariableVendor},
};

use crate::{BootResult, error::BootError};

/// The custom variable namespace for the boot manager.
const BOOTMGR_GUID: uefi::Guid = guid!("23600d08-561e-4e68-a024-1d7d6e04ee4e");

/// A trait for implementations of UEFI variable storage.
///
/// Usually this will use runtime services.
trait UefiVariableStorage {
    /// Get a variable given its name, a variable vendor, and a mutable byte slice.
    fn get_variable<T: UefiVariable + 'static>(
        name: &CStr16,
        vendor: &VariableVendor,
        buf: &mut [u8],
    ) -> BootResult<T>;

    /// Set a variable given its name, a variable vendor, variable attributes, and the chosen type.
    fn set_variable<T: UefiVariable + 'static>(
        name: &CStr16,
        vendor: &VariableVendor,
        attributes: VariableAttributes,
        num: Option<T>,
    ) -> BootResult<()>;
}

/// UEFI variable storage implementation with runtime services..
struct RuntimeUefiVariableStorage;

impl UefiVariableStorage for RuntimeUefiVariableStorage {
    fn get_variable<T: UefiVariable>(
        name: &CStr16,
        vendor: &VariableVendor,
        buf: &mut [u8],
    ) -> BootResult<T> {
        match runtime::get_variable(name, vendor, buf) {
            Ok((var, _)) => Ok(T::from_bytes(var)),
            Err(e) if e.status() == Status::NOT_FOUND => Ok(T::default()), // pretend that we got all zeroes if its not found
            Err(e) => Err(BootError::Uefi(e.to_err_without_payload())),
        }
    }

    fn set_variable<T: UefiVariable>(
        name: &CStr16,
        vendor: &VariableVendor,
        attributes: VariableAttributes,
        num: Option<T>,
    ) -> BootResult<()> {
        let num = num.map_or_else(|| Vec::with_capacity(0), UefiVariable::to_bytes);
        Ok(runtime::set_variable(name, vendor, attributes, &num)?)
    }
}

/// A value that can be stored in a UEFI variable.
///
/// This is essentially a type that can be converted into and from a vector of bytes. What byte ordering these bytes
/// are in does not particularly matter, or how these bytes are encoded or decoded, as long as the method from
/// [`UefiVariable`] is used instead of whatever type you have. It also has to be a set size.
pub trait UefiVariable: Sized {
    /// Convert `Self` to a vector of bytes.
    fn to_bytes(self) -> Vec<u8>;

    /// Convert a vector of bytes to `Self`.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Return 0, or an equivalent value.
    fn default() -> Self;
}

impl UefiVariable for usize {
    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u64 {
    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u32 {
    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u16 {
    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for u8 {
    fn to_bytes(self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        Self::from_le_bytes(array)
    }
    fn default() -> Self {
        0
    }
}

impl UefiVariable for bool {
    fn to_bytes(self) -> Vec<u8> {
        u8::from(self).to_le_bytes().to_vec()
    }
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut array = [0; size_of::<Self>()];
        array.copy_from_slice(bytes);
        u8::from_le_bytes(array) > 0
    }
    fn default() -> Self {
        false
    }
}

/// Sets a UEFI variable to a [`UefiVariable`] given the name.
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// This custom namespace is accessible at GUID `23600d08-561e-4e68-a024-1d7d6e04ee4e`.
///
/// Passing None for num will result in the variable being deleted.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::set_variable`]
pub fn set_variable<T: UefiVariable + 'static>(
    name: &CStr16,
    vendor: Option<VariableVendor>,
    attrs: Option<VariableAttributes>,
    num: Option<T>,
) -> BootResult<()> {
    let vendor = vendor.unwrap_or(runtime::VariableVendor(BOOTMGR_GUID));
    let attrs = attrs.map_or_else(
        || VariableAttributes::NON_VOLATILE | VariableAttributes::BOOTSERVICE_ACCESS,
        |x| x,
    );
    RuntimeUefiVariableStorage::set_variable(name, &vendor, attrs, num)
}

/// Gets a UEFI variable of a [`UefiVariable`] given the name
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// This custom namespace is accessible at GUID `23600d08-561e-4e68-a024-1d7d6e04ee4e`.
///
/// If the variable was not found, a default value of `0` will be returned. This is more convenient to handle
/// internally as its easier to not handle specially the case of the variable not being found.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::get_variable`]
pub fn get_variable<T: UefiVariable + 'static>(
    name: &CStr16,
    vendor: Option<VariableVendor>,
) -> BootResult<T> {
    let mut buf = vec![Default::default(); size_of::<T>()];
    let vendor = vendor.unwrap_or(runtime::VariableVendor(BOOTMGR_GUID));
    RuntimeUefiVariableStorage::get_variable(name, &vendor, &mut buf)
}

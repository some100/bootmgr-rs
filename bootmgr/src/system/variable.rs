// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! UEFI variable storage helpers.
//!
//! These store a value into a UEFI variable in a custom vendor namespace.

use alloc::string::String;

use thiserror::Error;
use uefi::{
    CStr16, Status, guid,
    runtime::{self, VariableAttributes, VariableVendor},
};

use crate::BootResult;

/// The custom variable namespace for the boot manager.
const BOOTMGR_GUID: uefi::Guid = guid!("23600d08-561e-4e68-a024-1d7d6e04ee4e");

/// The maximum size of a singular type to be stored in a UEFI variable in bytes.
const MAX_SIZE: usize = size_of::<u64>();

/// An `Error` that may result from attempting to get value from a UEFI variable.
#[derive(Error, Debug)]
pub enum VarError {
    /// The variable could not be obtained.
    #[error("Failed to get variable: {0}")]
    GetErr(#[from] uefi::Error<Option<usize>>),

    /// The string variable could not be cast into a u16.
    #[error("Failed to cast variable to u16: {0}")]
    CastErr(bytemuck::PodCastError),

    /// The variable did not contain a string with valid characters or a nul terminator.
    #[error("Failed to get string variable: {0}")]
    StrErr(#[from] uefi::data_types::FromSliceWithNulError),

    /// The string slice could not be converted into a UCS-2 string.
    #[error("Failed to convert string to UCS-2: {0}")]
    Ucs2ConvErr(#[from] uefi::data_types::FromStrWithBufError),
}

/// A trait for implementations of UEFI variable storage.
///
/// Usually this will use runtime services.
trait UefiVariableStorage {
    /// Get a variable given its name, a variable vendor, and a mutable byte slice.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the underlying variable storage failed to get the variable.
    fn get_variable<T: UefiVariable + 'static>(
        name: &CStr16,
        vendor: &VariableVendor,
    ) -> BootResult<T>;

    /// Set a variable given its name, a variable vendor, variable attributes, and the chosen type.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the underlying variable storage failed to set the variable.
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
    fn get_variable<T: UefiVariable>(name: &CStr16, vendor: &VariableVendor) -> BootResult<T> {
        let mut buf = [0; MAX_SIZE];
        match runtime::get_variable(name, vendor, &mut buf) {
            Ok((var, _)) => Ok(T::from_bytes(var)),
            Err(e) if e.status() == Status::NOT_FOUND => Ok(T::default()), // pretend that we got all zeroes if its not found
            Err(e) => Err(VarError::GetErr(e).into()),
        }
    }

    fn set_variable<T: UefiVariable>(
        name: &CStr16,
        vendor: &VariableVendor,
        attributes: VariableAttributes,
        num: Option<T>,
    ) -> BootResult<()> {
        let mut bytes = 0;
        let mut buf = [0; MAX_SIZE];
        if let Some(num) = num {
            bytes = num.to_bytes(&mut buf);
        }
        Ok(runtime::set_variable(
            name,
            vendor,
            attributes,
            &buf[0..bytes],
        )?)
    }
}

/// A value that can be stored in a UEFI variable.
///
/// This is essentially a type that can be converted into and from a vector of bytes. What byte ordering these bytes
/// are in does not particularly matter, or how these bytes are encoded or decoded, as long as the method from
/// [`UefiVariable`] is used instead of whatever type you have. It also has to be a set size.
pub trait UefiVariable: Sized {
    /// The size of the type in bytes.
    const SIZE: usize;

    /// Convert `Self` to a buffer of bytes, then return the bytes written.
    fn to_bytes(&self, out: &mut [u8]) -> usize;

    /// Convert a vector of bytes to `Self`.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Return 0, or an equivalent value.
    fn default() -> Self;
}

impl UefiVariable for usize {
    const SIZE: Self = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = self.to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
    const SIZE: usize = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = self.to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
    const SIZE: usize = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = self.to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
    const SIZE: usize = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = self.to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
    const SIZE: usize = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = self.to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
    const SIZE: usize = size_of::<Self>();

    fn to_bytes(&self, out: &mut [u8]) -> usize {
        let bytes = u8::from(*self).to_le_bytes();
        out[..bytes.len()].copy_from_slice(&bytes);
        bytes.len()
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
        || {
            VariableAttributes::NON_VOLATILE
                | VariableAttributes::BOOTSERVICE_ACCESS
                | VariableAttributes::RUNTIME_ACCESS
        },
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
    let vendor = vendor.unwrap_or(runtime::VariableVendor(BOOTMGR_GUID));
    RuntimeUefiVariableStorage::get_variable(name, &vendor)
}

/// Sets a UEFI variable to a [`u16`] slice given the name.
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// This custom namespace is accessible at GUID `23600d08-561e-4e68-a024-1d7d6e04ee4e`.
///
/// Passing None for str will result in the variable being deleted.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::set_variable`]
pub fn set_variable_u16_slice(
    name: &CStr16,
    vendor: Option<VariableVendor>,
    attrs: Option<VariableAttributes>,
    bytes: Option<&[u16]>,
) -> BootResult<()> {
    let vendor = vendor.unwrap_or(runtime::VariableVendor(BOOTMGR_GUID));
    let attrs = attrs.map_or_else(
        || {
            VariableAttributes::NON_VOLATILE
                | VariableAttributes::BOOTSERVICE_ACCESS
                | VariableAttributes::RUNTIME_ACCESS
        },
        |x| x,
    );
    let bytes = bytes.unwrap_or(&[] as &[u16]);
    Ok(runtime::set_variable(
        name,
        &vendor,
        attrs,
        bytemuck::must_cast_slice(bytes),
    )?)
}

/// Sets a UEFI variable to a [`str`] slice given the name.
///
/// This is another convenience wrapper around [`set_variable_cstr`]. This converts
/// a [`str`] slice into a [`CStr16`] slice, which is converted into a [`u16`] slice
/// before setting the variable.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::set_variable`]
pub fn set_variable_str(
    name: &CStr16,
    vendor: Option<VariableVendor>,
    attrs: Option<VariableAttributes>,
    str: Option<&str>,
) -> BootResult<()> {
    let mut buf = [0; 256];
    let str = if let Some(str) = str {
        Some(
            CStr16::from_str_with_buf(str, &mut buf)
                .map_err(VarError::Ucs2ConvErr)?
                .to_u16_slice_with_nul(),
        )
    } else {
        None
    };
    set_variable_u16_slice(name, vendor, attrs, str)
}

/// Gets a UEFI variable of a [`str`] slice given the name
///
/// If None is specified for the vendor, then the variable will be searched for in a custom GUID space,
/// not the global variables vendor space. In other words, unless you are storing your own variables,
/// it may not be what you expect.
///
/// This custom namespace is accessible at GUID `23600d08-561e-4e68-a024-1d7d6e04ee4e`.
///
/// If the variable was not found, an empty string will be returned.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`runtime::get_variable`]. In addition if the variable could not be
/// converted into a u16 slice, or the variable could not be converted into a [`CString16`], then an error will be
/// returned.
pub fn get_variable_str(name: &CStr16, vendor: Option<VariableVendor>) -> BootResult<String> {
    let vendor = vendor.unwrap_or(runtime::VariableVendor(BOOTMGR_GUID));
    let var = match runtime::get_variable_boxed(name, &vendor) {
        Ok((var, _)) => var,
        Err(e) if e.status() == Status::NOT_FOUND => return Ok(String::new()),
        Err(e) => return Err(e.into()),
    };
    let str = bytemuck::try_cast_slice(&var).map_err(VarError::CastErr)?;

    Ok(String::from(
        CStr16::from_u16_with_nul(str).map_err(VarError::StrErr)?,
    ))
}

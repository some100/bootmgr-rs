//! Various helper functions for other modules.

use core::mem::MaybeUninit;

use alloc::ffi::CString;

use alloc::string::String;
use smallvec::SmallVec;
use thiserror::Error;
use uefi::CStr8;
use uefi::{
    CStr16, CString16, boot,
    data_types::PoolString,
    proto::device_path::{
        DevicePath, PoolDevicePath, build,
        text::{AllowShortcuts, DevicePathToText, DisplayOnly},
    },
};

use crate::{BootResult, config::types::Architecture};

/// The length of a BLS machine-id.
const MACHINE_ID_LEN: usize = 32;

/// The max length of a path in UEFI.
const MAX_PATH: usize = 256;

/// An `Error` that may result from converting a [`String`] to another format.
#[derive(Error, Debug)]
pub enum StrError {
    /// A [`String`] could not be converted into a [`CString16`]
    #[error("Could not convert String to CString16")]
    CstrFromStr(#[from] uefi::data_types::FromStrError),

    /// A byte slice could not be converted into a [`CString`], due to an invalid
    /// character or nul character found.
    #[error("Could not convert a byte slice to a CString*")]
    FromSliceWithNul(#[from] uefi::data_types::FromSliceWithNulError),

    /// A [`String`] could not be converted into a [`CString`]
    #[error("Could not convert String to CString")]
    CstringFromStr(#[from] alloc::ffi::NulError),
}

/// An `Error` that may result from building a [`DevicePath`]
#[derive(Error, Debug)]
pub enum DevicePathError {
    /// A Device Path could not be built. This can if the buffer was too small.
    #[error("Could not build DevicePath")]
    Build(#[from] uefi::proto::device_path::build::BuildError),

    /// The Device Path could not be appended to an existing one for some reason.
    #[error("Could not append DevicePath to another DevicePath")]
    DevPathUtil(#[from] uefi::proto::device_path::DevicePathUtilitiesError),
}

/// Tests if a sort key is valid.
///
/// Returns true if every character is ASCII alphanumeric, or a `.`, or an `_`, or a `-`. Otherwise,
/// will return false.
#[must_use = "Has no effect if the result is unused"]
pub(crate) fn check_sort_key_valid(sort_key: &str) -> bool {
    sort_key
        .chars()
        .all(|x| x.is_ascii_alphanumeric() || x == '.' || x == '_' || x == '-')
}

/// Tests if a machine id is valid.
///
/// Returns true if the character count is exactly 32 characters in length, and every character is a hex
/// digit. Otherwise, will return false.
#[must_use = "Has no effect if the result is unused"]
pub(crate) fn check_machine_id_valid(machine_id: &str) -> bool {
    machine_id.chars().count() == MACHINE_ID_LEN
        && machine_id.chars().all(|x| x.is_ascii_hexdigit())
}

/// Converts a [`DevicePath`] into a text representation.
///
/// # Errors
///
/// May return an `Error` if the system does not support [`DevicePathToText`], or there is not enough memory.
pub(crate) fn device_path_to_text(device_path: &DevicePath) -> BootResult<PoolString> {
    let handle = boot::get_handle_for_protocol::<DevicePathToText>()?;
    let device_path_to_text = boot::open_protocol_exclusive::<DevicePathToText>(handle)?;
    Ok(device_path_to_text.convert_device_path_to_text(
        device_path,
        DisplayOnly(true),
        AllowShortcuts(false),
    )?)
}

/// Gets a [`CString16`] from an [`&str`].
///
/// # Errors
///
/// May return an `Error` if the string could not be converted into a [`CString16`], either due to unsupported
/// characters or an invalid nul character.
pub(crate) fn str_to_cstr(str: &str) -> Result<CString16, StrError> {
    Ok(CString16::try_from(str)?)
}

/// Gets a [`CString16`] path given a prefix and a filename.
///
/// # Errors
///
/// May return an `Error` if the finalized string could not be converted into a [`CString16`]. This should be
/// impossible because of the fact that validation is already done through the parameters being [`CStr16`].
pub(crate) fn get_path_cstr(prefix: &CStr16, filename: &CStr16) -> Result<CString16, StrError> {
    let mut path_buf: SmallVec<[_; MAX_PATH]> =
        SmallVec::with_capacity(prefix.as_slice().len() + 1 + filename.as_slice().len());

    path_buf.extend_from_slice(prefix.to_u16_slice());
    path_buf.push(u16::from(b'\\'));
    path_buf.extend_from_slice(filename.to_u16_slice_with_nul());

    Ok(CStr16::from_u16_with_nul(&path_buf)?.into())
}

/// Gets a [`CString`] from an [`&str`].
///
/// Not to be confused with a [`CString16`].
///
/// # Errors
///
/// May return an `Error` if the string could not be converted into a [`CString`] because an interior
/// nul character was found.
pub(crate) fn str_to_cstring(str: &str) -> Result<CString, StrError> {
    Ok(CString::new(str)?)
}

/// Gets a [`CStr8`] from a byte slice containing UTF-8.
///
/// # Errors
///
/// May return an `Error` if the bytes could not be converted into a [`CStr8`] because an interior nul
/// character was found, or there was an invalid character.
pub(crate) fn bytes_to_cstr8(bytes: &[u8]) -> Result<&CStr8, StrError> {
    Ok(CStr8::from_bytes_with_nul(bytes)?)
}

/// Gets the target architecture of the bootloader binary.
#[must_use = "Has no effect if the result is unused"]
pub fn get_arch() -> Option<Architecture> {
    if cfg!(target_arch = "x86") {
        Architecture::new("x86").ok()
    } else if cfg!(target_arch = "x86_64") {
        Architecture::new("x64").ok()
    } else if cfg!(target_arch = "arm") {
        Architecture::new("arm").ok()
    } else if cfg!(target_arch = "aarch64") {
        Architecture::new("aa64").ok()
    } else {
        None // rust doesnt support itanium anyways
    }
}

/// Gets the joined [`DevicePath`] given an existing [`DevicePath`] (likely to a partition) and a file's path.
///
/// The provided mutable buffer must be large enough to fit the final [`DevicePath`].
///
/// # Errors
///
/// May return an `Error` if the device path is finalized before the file's [`DevicePath`] could be pushed.
/// Though, this should be quite unlikely.
pub(crate) fn join_to_device_path(
    dev_path: &DevicePath,
    path: &CStr16,
    buf: &mut [u8],
) -> Result<PoolDevicePath, DevicePathError> {
    let buf = slice_to_maybe_uninit(buf);
    let path: &DevicePath = build::DevicePathBuilder::with_buf(buf)
        .push(&build::media::FilePath { path_name: path })?
        .finalize()?;
    Ok(dev_path.append_path(path)?)
}

/// Normalizes a path to make it more aligned with UEFI expectations
///
/// Currently this means replacing all forward slashes with backslashes.
#[must_use = "Has no effect if the result is unused"]
pub(crate) fn normalize_path(path: &str) -> String {
    path.replace('/', "\\")
}

/// Converts a byte slice into an `&mut [MaybeUninit<u8>]`.
pub(crate) fn slice_to_maybe_uninit(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    // SAFETY: this is essentially equivalent to reconstructing an &mut [MaybeUninit<u8>] from a mutable slice.
    // because slices are always valid as pointers, and the length of the two slices are the same, this is safe.
    unsafe {
        core::slice::from_raw_parts_mut(slice.as_mut_ptr().cast::<MaybeUninit<u8>>(), slice.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uefi::cstr16;

    #[test]
    fn test_check_sort_key_valid() {
        let sort_key = "sort-key";
        assert!(check_sort_key_valid(sort_key));
        let sort_key = "super Invalid ;; sort key sssz.";
        assert!(!check_sort_key_valid(sort_key));
    }

    #[test]
    fn test_check_machine_id_valid() {
        let machine_id = "1234567890abcdef1234567890abcdef";
        assert!(check_machine_id_valid(machine_id));
        let machine_id = "1234567890abcdef1234567890abcdeg";
        assert!(!check_machine_id_valid(machine_id));
        let machine_id = "obviously invalid";
        assert!(!check_machine_id_valid(machine_id));
    }

    #[test]
    fn test_str_to_cstr() -> Result<(), StrError> {
        let cstr = str_to_cstr("foo bar")?;
        let str = String::from(&cstr);
        assert_eq!(str, "foo bar".to_owned());
        Ok(())
    }

    #[test]
    fn test_get_path_cstr() -> Result<(), StrError> {
        const PREFIX: &CStr16 = cstr16!("\\root");
        const FILE: &CStr16 = cstr16!("somefilename");
        let path = get_path_cstr(PREFIX, FILE)?;
        let str = String::from(&path);
        assert_eq!(str, "\\root\\somefilename".to_owned());
        Ok(())
    }

    #[test]
    fn test_get_arch() {
        if cfg!(target_arch = "x86") {
            assert_eq!(get_arch().as_deref().map(String::as_str), Some("x86"));
        } else if cfg!(target_arch = "x86_64") {
            assert_eq!(get_arch().as_deref().map(String::as_str), Some("x64"));
        } else if cfg!(target_arch = "arm") {
            assert_eq!(get_arch().as_deref().map(String::as_str), Some("arm"));
        } else if cfg!(target_arch = "aarch64") {
            assert_eq!(get_arch().as_deref().map(String::as_str), Some("aa64"));
        } else {
            assert_eq!(get_arch(), None);
        }
    }

    #[test]
    fn test_normalize_path() {
        let path = "/some/path/from/linux/fs";
        assert_eq!(normalize_path(path), "\\some\\path\\from\\linux\\fs");
        let path = "\\a\\completely\\normal\\path";
        assert_eq!(normalize_path(path), path);
    }
}

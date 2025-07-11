//! Various helper functions for other modules

use alloc::{borrow::ToOwned, string::String, vec::Vec};
use uefi::{
    CStr16, CString16,
    proto::device_path::{DevicePath, PoolDevicePath, build},
};

use crate::error::BootError;

/// Gets a [`CString16`] from an [`&str`].
#[must_use]
pub fn str_to_cstr(str: &str) -> CString16 {
    match CString16::try_from(str) {
        Ok(str) => str,
        Err(_) => CString16::new(), // this is wrong, but at least it won't panic
    }
}

/// Gets a [`CString16`] path given a prefix and a filename.
#[must_use]
pub fn get_path_cstr(prefix: &CStr16, filename: &CStr16) -> CString16 {
    let mut path_buf = Vec::with_capacity(prefix.as_slice().len() + 1 + filename.as_slice().len());

    path_buf.extend_from_slice(prefix.to_u16_slice());
    path_buf.push(u16::from(b'\\'));
    path_buf.extend_from_slice(filename.to_u16_slice_with_nul());
    CString16::try_from(path_buf).unwrap_or_else(|_| CString16::new()) // this is wrong, but at least it won't panic
}

/// Gets the target architecture of the bootloader binary.
#[must_use]
pub fn get_arch() -> Option<String> {
    if cfg!(target_arch = "x86") {
        Some("x86".to_owned())
    } else if cfg!(target_arch = "x86_64") {
        Some("x64".to_owned())
    } else if cfg!(target_arch = "arm") {
        Some("arm".to_owned())
    } else if cfg!(target_arch = "aarch64") {
        Some("aa64".to_owned())
    } else {
        None // rust doesnt support itanium anyways
    }
}

/// Gets the joined [`DevicePath`] given an existing [`DevicePath`] (likely to a partition) and a file's path.
///
/// # Errors
///
/// May return an `Error` if the device path is finalized before the file's [`DevicePath`] could be pushed.
/// Though, this should be quite unlikely.
pub fn get_device_path(
    dev_path: &DevicePath,
    path: &CStr16,
    vec: &mut Vec<u8>,
) -> Result<PoolDevicePath, BootError> {
    let path: &DevicePath = build::DevicePathBuilder::with_vec(vec)
        .push(&build::media::FilePath { path_name: path })?
        .finalize()?;
    Ok(dev_path.append_path(path)?)
}

/// Normalizes a path to make it more aligned with UEFI expectations
///
/// Currently this means replacing all forward slashes with backslashes.
#[must_use]
pub fn normalize_path(path: &str) -> String {
    path.replace('/', "\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uefi::cstr16;

    #[test]
    fn test_str_to_cstr() {
        let cstr = str_to_cstr("foo bar");
        let str = String::from(&cstr);
        assert_eq!(str, "foo bar".to_owned());
    }

    #[test]
    fn test_get_path_cstr() {
        const PREFIX: &CStr16 = cstr16!("\\root");
        const FILE: &CStr16 = cstr16!("somefilename");
        let path = get_path_cstr(PREFIX, FILE);
        let str = String::from(&path);
        assert_eq!(str, "\\root\\somefilename".to_owned());
    }

    #[test]
    fn test_get_arch() {
        if cfg!(target_arch = "x86") {
            assert_eq!(get_arch().expect("get_arch returns None for x86"), "x86".to_owned());
        } else if cfg!(target_arch = "x86_64") {
            assert_eq!(get_arch().expect("get_arch returns None for x64"), "x64".to_owned());
        } else if cfg!(target_arch = "arm") {
            assert_eq!(get_arch().expect("get_arch returns None for arm"), "arm".to_owned());
        } else if cfg!(target_arch = "aarch64") {
            assert_eq!(get_arch().expect("get_arch returns None for aa64"), "aa64".to_owned());
        } else {
            assert_eq!(get_arch(), None);
        }
    }

    #[test]
    fn test_normalize_path() {
        let path = "/some/path/from/linux/fs";
        assert_eq!(normalize_path(path), "\\some\\path\\from\\linux\\fs");
    }
}

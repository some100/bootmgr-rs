//! Security hooks that attach to their respective protocols.
//!
//! Secure Boot interacts with its validators by calling upon a method stored within the [`SecurityArchProtocol`] and
//! [`Security2ArchProtocol`]. Respectively, these are the `auth_state` and `authentication` methods. We can replace
//! these methods with our own custom security hooks in case we need to use a custom validator, like with Shim.
//!
//! These security hooks are very simple in nature, and follow a series of steps:
//! 1. Take the raw pointers passed to the hooks, and parse them as safer equivalents ([`Option<DevicePath>`], `&mut [u8]`)
//! 2. Pass those safer equivalents to the custom validator
//! 3. If the validator returns a failed status, then pass those raw pointers to the original validators.
//!
//! It will also provide an implementation for `SecurityOverrideInner` for installing those hooks into the security override
//! state.
//!
//! # Safety
//!
//! This module uses unsafe in 5 places. This is quite dangerous, though in the context of how these security hooks are called,
//! it should still be quite safe even considering the risks.
//!
//! 1. Unsafe is required to call FFI functions such as the original hook. This requires one condition, which is upheld in the
//!    normal calling context of the program. This is that the raw pointers must not be invalid. This is unavoidable even if
//!    the security override was not installed, since this would indicate a problem with the firmware.
//! 2. Unsafe is required to convert mutable pointers to u8 slices. This requires one condition, which is upheld in the normal
//!    calling context of the program. If the size that was passed to the hook was inaccurate, then it will result in UB. This,
//!    yet again, is unavoidable and would indicate a problem with firmware.
//! 3. Unsafe is required to call FFI functions such as the original hook. See point 1.
//! 4. Unsafe is required to convert mutable pointers to u8 slices. See point 2.
//! 5. Unsafe is required to convert FFI [`DevicePath`]s into regular [`DevicePath`]s. However, this should be safe as long as
//!    the data is valid (which is true in the normal calling context of the program). There are checks to see if the pointer is
//!    non-null and aligned as well. In addition, the pointer is not modified at all as it is passed as `*const`.

use core::ffi::c_void;

use log::warn;
use uefi::{
    Status, boot,
    proto::device_path::{DevicePath, FfiDevicePath},
};

use crate::{
    boot::secure_boot::{SECURITY_OVERRIDE, security_override::SecurityOverrideInner},
    system::protos::{Security2Arch, Security2ArchProtocol, SecurityArch, SecurityArchProtocol},
};

impl SecurityOverrideInner {
    /// Installs the security hook for [`SecurityArch`].
    ///
    /// It will only install the hook if the firmware supports [`SecurityArch`].
    pub(super) fn install_security1_hook(&mut self) {
        if let Ok(handle) = boot::get_handle_for_protocol::<SecurityArch>()
            && let Ok(mut security) = boot::open_protocol_exclusive::<SecurityArch>(handle)
        {
            security.get_inner_mut().auth_state = auth_state_hook;
            self.original_hook = Some(security.get_inner().auth_state);
            self.security = Some(handle);
        }
    }

    /// Installs the security hook for [`Security2Arch`].
    ///
    /// It will only install the hook if the firmware supports [`Security2Arch`].
    pub(super) fn install_security2_hook(&mut self) {
        if let Ok(handle) = boot::get_handle_for_protocol::<Security2Arch>()
            && let Ok(mut security) = boot::open_protocol_exclusive::<Security2Arch>(handle)
        {
            security.get_inner_mut().authentication = authentication_hook;
            self.original_hook2 = Some(security.get_inner().authentication);
            self.security2 = Some(handle);
        }
    }

    /// Uninstalls the security hook for [`SecurityArch`].
    ///
    /// Three conditions must be true:
    /// - Original hook installed in struct
    /// - [`SecurityArch`] `Handle` present in struct
    /// - Firmware supports [`SecurityArch`].
    ///
    /// Otherwise, this method will do nothing.
    pub(super) fn uninstall_security1_hook(&self) {
        if let Some(original_hook) = self.original_hook
            && let Some(handle) = self.security
            && let Ok(mut security) = boot::open_protocol_exclusive::<SecurityArch>(handle)
        {
            security.get_inner_mut().auth_state = original_hook;
        }
    }

    /// Uninstalls the security hook for [`Security2Arch`].
    ///
    /// Three conditions must be true:
    /// - Original hook installed in struct
    /// - [`Security2Arch`] `Handle` present in struct
    /// - Firmware supports [`Security2Arch`].
    ///
    /// Otherwise, this method will do nothing.
    pub(super) fn uninstall_security2_hook(&self) {
        if let Some(original_hook2) = self.original_hook2
            && let Some(handle) = self.security2
            && let Ok(mut security) = boot::open_protocol_exclusive::<Security2Arch>(handle)
        {
            security.get_inner_mut().authentication = original_hook2;
        }
    }
}

/// The override hook for [`SecurityArchProtocol`].
///
/// This calls the custom validator to validate the `file` parameter. If the validator fails, then the original hook
/// will be used to verify the image.
///
/// # Safety
///
/// The parameters to this function take raw pointers. The caller must ensure that the pointers are valid, and non null.
/// Even then, it should still be relatively safe because of checks for invalid pointers.
unsafe extern "efiapi" fn auth_state_hook(
    this: *const SecurityArchProtocol,
    auth_status: u32,
    file: *const FfiDevicePath,
) -> Status {
    let security_override = &SECURITY_OVERRIDE;

    match security_override
        .get()
        .call_validator(ffi_ptr_to_device_path(file), None)
    {
        Err(e) => {
            warn!("{e}"); // if we get an error, log it and call the original hook to be the final verdict

            // SAFETY: if UEFI LoadImage is calling this hook, these arguments should be completely valid and safe
            unsafe {
                security_override
                    .get()
                    .call_original_hook(this, auth_status, file)
            }
        }
        _ => Status::SUCCESS, // if there was no error, return success (the image is valid)
    }
}

/// The override hook for [`Security2ArchProtocol`].
///
/// This calls the custom validator to validate the either the `device_path` or `file_buffer` parameters. If the
/// validator fails, then the original hook will be used to verify the image.
///
/// # Safety
///
/// The parameters to this function take raw pointers. The caller must ensure that the pointers are valid, and non null.
/// Even then, it should still be relatively safe because of checks for invalid pointers. However, if `file_size` is not
/// the exact length of `file_buffer`, then undefined behavior will result. In basically every case, firmware will call
/// this function with the correct values.
///
/// If the firmware passed incorrect values, that would be a catastrophic issue on the firmware's side, and probably would
/// not even boot at all with Secure Boot enabled.
unsafe extern "efiapi" fn authentication_hook(
    this: *const Security2ArchProtocol,
    device_path: *const FfiDevicePath,
    file_buffer: *mut c_void,
    file_size: usize,
    boot_policy: u8,
) -> Status {
    let security_override = &SECURITY_OVERRIDE;

    // SAFETY: this is quite unsafe as file_size is not guaranteed to be the same size as file_buffer.
    // if UEFI LoadImage is calling this hook, however, it should be safe, since there are no other
    // direct users.
    let slice = unsafe { mut_ptr_to_u8_slice(file_buffer, file_size) };

    match security_override
        .get()
        .call_validator(ffi_ptr_to_device_path(device_path), slice)
    {
        Err(e) => {
            warn!("{e}"); // if we get an error, log it and call the original hook to be the final verdict

            // SAFETY: if UEFI LoadImage is calling this hook, these arguments should be completely valid and safe
            unsafe {
                security_override.get().call_original_hook2(
                    this,
                    device_path,
                    file_buffer,
                    file_size,
                    boot_policy,
                )
            }
        }
        _ => Status::SUCCESS, // if there was no error, return success (the image is valid)
    }
}

/// Convert a mutable raw [`c_void`] to a mutable byte slice.
///
/// If the [`c_void`] is an invalid pointer, then it will return [`None`]. However, this is still unsafe as
/// the size passed through the parameter cannot be verified as the exact size of the slice.
unsafe fn mut_ptr_to_u8_slice<'a>(ptr: *mut c_void, size: usize) -> Option<&'a mut [u8]> {
    // SAFETY: an invalid pointer should be guarded against, but if the size is invalid, then this
    // could be unsafe.
    (!ptr.is_null() && ptr.is_aligned())
        .then(|| unsafe { core::slice::from_raw_parts_mut(ptr.cast::<u8>(), size) })
}

/// Convert an [`FfiDevicePath`] to a [`DevicePath`].
///
/// If [`FfiDevicePath`] is an invalid pointer, then it will return [`None`]. Because of this, this function is safe.
fn ffi_ptr_to_device_path<'a>(ptr: *const FfiDevicePath) -> Option<&'a DevicePath> {
    // SAFETY: this checks the pointer for validity beforehand, so the pointer should be valid and therefore safe
    (!ptr.is_null() && ptr.is_aligned()).then(|| unsafe { DevicePath::from_ffi_ptr(ptr) })
}

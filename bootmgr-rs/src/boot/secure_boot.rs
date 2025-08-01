//! Secure Boot support module.
//!
//! Secure Boot interacts with the loading of images through the `SecurityArch` and
//! `Security2Arch` protocol. In firmware, if any of these two are published, `LoadImage`
//! must use those protocols on every image that is loaded. The `Security2Arch` protocol
//! takes priority.
//!
//! Internally, the `FileAuthentication` and `FileAuthenticationState` methods are used for
//! verifying images. These methods can also be replaced with our own custom made ones, mainly
//! also using Shim.
//!
//! This hooks onto `SecurityArch` and `Security2Arch` in order to replace their
//! authenticators with custom ones using Shim or any other validator.
//!
//! These hooks are temporary and should be uninstalled after the image is loaded.

use core::cell::Cell;
use core::{ffi::c_void, ptr::NonNull};

use log::warn;
use thiserror::Error;
use uefi::{
    Status, cstr16,
    proto::device_path::{DevicePath, FfiDevicePath},
    runtime::VariableVendor,
};

use crate::{
    BootResult,
    boot::secure_boot::security_override::SecurityOverrideInner,
    system::{
        protos::{Security2ArchProtocol, SecurityArchProtocol},
        variable::get_variable,
    },
};

pub mod security_override;
pub mod shim;

/// An `Error` that may result from validating an image with Secure Boot.
#[derive(Error, Debug)]
pub enum SecureBootError {
    /// Neither a device path nor a file buffer were specified to the image.
    #[error("DevicePath and file buffer were both None")]
    NoDevicePathOrFile,
    /// A validator was not installed, but the security hooks were installed.
    #[error("Validator was not installed")]
    NoValidator,
}

/// The type alias for the [`SecurityArchProtocol`] `auth_state` function.
///
/// Should probably not be used directly.
type AuthState = unsafe extern "efiapi" fn(
    this: *const SecurityArchProtocol,
    auth_status: u32,
    file: *const FfiDevicePath,
) -> Status;

/// The type alias for the [`Security2ArchProtocol`] `authentication` function.
///
/// Should probably not be used directly.
type Authentication = unsafe extern "efiapi" fn(
    this: *const Security2ArchProtocol,
    device_path: *const FfiDevicePath,
    file_buffer: *mut c_void,
    file_size: usize,
    boot_policy: u8,
) -> Status;

/// The function signature for a validator.
pub type Validator = fn(
    ctx: Option<NonNull<u8>>,
    device_path: Option<&DevicePath>,
    file_buffer: Option<&mut [u8]>,
    file_size: usize,
) -> BootResult<()>;

/// An instance of [`SecurityOverrideInner`] that lasts for the lifetime of the program.
///
/// This is required because of how the security hooks that are installed do not have a usable context field,
/// so we cannot simply supply the inner security override as that context. Instead, we have a static instance
/// of the [`SecurityOverrideInner`] that the security hooks may access.
static SECURITY_OVERRIDE: SecurityOverride = SecurityOverride {
    inner: Cell::new(None),
};

/// The security override, for installing a custom validator.
pub struct SecurityOverride {
    /// The inner [`SecurityOverrideInner`] wrapped around a [`Cell`] for safety.
    inner: Cell<Option<SecurityOverrideInner>>,
}

impl SecurityOverride {
    /// Return a copy of the inner [`SecurityOverrideInner`].
    ///
    /// This will panic if the [`Cell`] is not yet initialized.
    fn get(&self) -> SecurityOverrideInner {
        self.inner.get().expect("Secure Boot Cell not initialized")
    }
}

// SAFETY: uefi is a single threaded environment there is no notion of thread safety
unsafe impl Sync for SecurityOverride {}

/// Tests if secure boot is enabled through a UEFI variable.
#[must_use = "Has no effect if the result is unused"]
pub fn secure_boot_enabled() -> bool {
    matches!(
        get_variable::<u8>(cstr16!("SecureBoot"), Some(VariableVendor::GLOBAL_VARIABLE)),
        Ok(1)
    )
}

/// Installs a security override given a [`Validator`] and optionally a `validator_ctx`.
pub fn install_security_override(validator: Validator, validator_ctx: Option<NonNull<u8>>) {
    let security_override = &SECURITY_OVERRIDE;
    let mut inner = SecurityOverrideInner::default();
    inner.install_validator(validator, validator_ctx);

    security_override.inner.set(Some(inner));
}

/// Uninstalls the security override. Should be used after installing the security override.
pub fn uninstall_security_override() {
    let security_override = &SECURITY_OVERRIDE;

    security_override.get().uninstall_validator();
    security_override.inner.take();
}

/// The override hook for [`SecurityArchProtocol`].
///
/// This calls the custom validator to validate the `file` parameter. If the validator fails, then the original hook
/// will be used to verify the image.
unsafe extern "efiapi" fn security_hook(
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
            warn!("{e}");
            unsafe {
                security_override
                    .get()
                    .call_original_hook(this, auth_status, file)
            }
        }
        _ => Status::SUCCESS,
    }
}

/// The override hook for [`Security2ArchProtocol`].
///
/// This calls the custom validator to validate the either the `device_path` or `file_buffer` parameters. If the
/// validator fails, then the original hook will be used to verify the image.
unsafe extern "efiapi" fn security2_hook(
    this: *const Security2ArchProtocol,
    device_path: *const FfiDevicePath,
    file_buffer: *mut c_void,
    file_size: usize,
    boot_policy: u8,
) -> Status {
    let security_override = &SECURITY_OVERRIDE;

    let file_slice =
        unsafe { core::slice::from_raw_parts_mut(file_buffer.cast::<u8>(), file_size) };
    match security_override
        .get()
        .call_validator(ffi_ptr_to_device_path(device_path), Some(file_slice))
    {
        Err(e) => {
            warn!("{e}");
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
        _ => Status::SUCCESS,
    }
}

/// Convert an [`FfiDevicePath`] to a [`DevicePath`].
///
/// If [`FfiDevicePath`] is null, then it will return [`None`].
fn ffi_ptr_to_device_path<'a>(ptr: *const FfiDevicePath) -> Option<&'a DevicePath> {
    (!ptr.is_null()).then(|| unsafe { DevicePath::from_ffi_ptr(ptr) })
}

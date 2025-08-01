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
//! These hooks are temporary and should be uninstalled after the image is loaded. This is done
//! automatically through the [`SecurityOverrideGuard`] struct.

use core::cell::Cell;
use core::ptr::NonNull;

use thiserror::Error;
use uefi::{cstr16, proto::device_path::DevicePath, runtime::VariableVendor};

use crate::{
    BootResult, boot::secure_boot::security_override::SecurityOverrideInner,
    system::variable::get_variable,
};

pub mod security_hooks;
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

/// A guard for [`SecurityOverride`]. When created, it will install a validator. When the
/// override is eventually dropped, the validator will be uninstalled.
pub struct SecurityOverrideGuard {
    /// If the validator is installed or not.
    installed: bool,
}

impl SecurityOverrideGuard {
    /// Create a new [`SecurityOverrideGuard`]. Installs a validator and returns the guard.
    ///
    /// When the returned guard is dropped, the security override is automatically uninstalled.
    pub fn new(validator: Validator, validator_ctx: Option<NonNull<u8>>) -> Self {
        install_security_override(validator, validator_ctx);
        Self { installed: true }
    }
}

impl Drop for SecurityOverrideGuard {
    fn drop(&mut self) {
        if self.installed {
            uninstall_security_override();
        }
    }
}

/// Tests if secure boot is enabled through a UEFI variable.
#[must_use = "Has no effect if the result is unused"]
pub fn secure_boot_enabled() -> bool {
    matches!(
        get_variable::<u8>(cstr16!("SecureBoot"), Some(VariableVendor::GLOBAL_VARIABLE)),
        Ok(1)
    )
}

/// Installs a security override given a [`Validator`] and optionally a `validator_ctx`.
///
/// Alternatively, you can use the [`SecurityOverrideGuard`] to safely ensure the override is dropped.
pub fn install_security_override(validator: Validator, validator_ctx: Option<NonNull<u8>>) {
    let security_override = &SECURITY_OVERRIDE;
    let mut inner = SecurityOverrideInner::default();
    inner.install_validator(validator, validator_ctx);

    security_override.inner.set(Some(inner));
}

/// Uninstalls the security override. Should be used after installing the security override.
///
/// Alternatively, you can use the [`SecurityOverrideGuard`] to safely ensure the override is dropped.
pub fn uninstall_security_override() {
    let security_override = &SECURITY_OVERRIDE;

    security_override.get().uninstall_validator();
    security_override.inner.take();
}

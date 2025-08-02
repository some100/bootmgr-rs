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
//! Even though Shim is the main consumer of this type of module, the overall architecture is
//! very pluggable and custom validators not simply delegating to Shim can be used as well.
//!
//! This hooks onto `SecurityArch` and `Security2Arch` in order to replace their
//! authenticators with custom ones using Shim or any other validator.
//!
//! These hooks are temporary and should be uninstalled after the image is loaded. This is done
//! automatically through the [`SecurityOverrideGuard`] struct.

use core::cell::OnceCell;
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
    /// The security override was already installed.
    #[error("Security override already installed")]
    AlreadyInstalled,
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
/// This is mandatory mainly due to the security hooks. We cannot provide arbitrary context to the security hooks, as the
/// function signature is constant and decided by the UEFI spec. Without a global static state, the security hooks cannot
/// possibly know of the existence of what custom validator we have installed, or where the original security validators
/// are.
///
/// To partially counter the security risk that a global static state brings, the inner override may only be set a grand
/// total of one time, due to it using a [`OnceCell`]. This makes it so that it cannot be modified after the security
/// override is installed.
static SECURITY_OVERRIDE: SecurityOverride = SecurityOverride {
    inner: OnceCell::new(),
};

/// The security override, for installing a custom validator.
pub struct SecurityOverride {
    /// The inner [`SecurityOverrideInner`] wrapped around a [`OnceCell`] for safety.
    inner: OnceCell<SecurityOverrideInner>,
}

impl SecurityOverride {
    /// Return a reference to the inner [`SecurityOverrideInner`].
    ///
    /// This will panic if the [`OnceCell`] is not yet initialized.
    /// However, this is not possible since the [`OnceCell`] is always initalized at the start
    /// of the program as a static. Therefore, this method cannot actually panic.
    fn get(&self) -> &SecurityOverrideInner {
        self.inner.get().unwrap() // even though unwrap is used here, this cannot panic
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
    ///
    /// # Errors
    ///
    /// May return an `Error` if the security override was already installed before.
    pub fn new(
        validator: Validator,
        validator_ctx: Option<NonNull<u8>>,
    ) -> Result<Self, SecureBootError> {
        install_security_override(validator, validator_ctx)?;
        Ok(Self { installed: true })
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
///
/// # Errors
///
/// May return an `Error` if the security override was already installed before.
pub fn install_security_override(
    validator: Validator,
    validator_ctx: Option<NonNull<u8>>,
) -> Result<(), SecureBootError> {
    let security_override = &SECURITY_OVERRIDE;
    let mut inner = SecurityOverrideInner::default();
    inner.install_validator(validator, validator_ctx);

    security_override
        .inner
        .set(inner)
        .map_err(|_| SecureBootError::AlreadyInstalled)?;
    Ok(())
}

/// Uninstalls the security override. Should be used after installing the security override.
///
/// Alternatively, you can use the [`SecurityOverrideGuard`] to safely ensure the override is dropped.
pub fn uninstall_security_override() {
    let security_override = &SECURITY_OVERRIDE;

    security_override.get().uninstall_validator();
}

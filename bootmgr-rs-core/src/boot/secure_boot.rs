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
//! automatically through the `SecurityOverrideGuard` struct.

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
pub(super) type Validator = fn(
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
/// To partially counter the risk that a singular static state brings, this static is not exposed to anywhere other than
/// [`SecurityOverrideGuard`]. This may need to be changed more than once in case `LoadImage` fails, and the override
/// is still installed.
static SECURITY_OVERRIDE: SecurityOverride = SecurityOverride {
    inner: Cell::new(None),
};

/// The security override, for installing a custom validator.
struct SecurityOverride {
    /// The inner [`SecurityOverrideInner`] wrapped around a [`Cell`] for safety.
    inner: Cell<Option<SecurityOverrideInner>>,
}

impl SecurityOverride {
    /// Return a copy of the inner [`SecurityOverrideInner`].
    ///
    /// This will panic if the [`Cell`] is not yet initialized.
    /// However, this is not possible since the [`Cell`] is always initalized at the start
    /// of the program as a static. Therefore, this method cannot actually panic.
    const fn get(&self) -> SecurityOverrideInner {
        self.inner
            .get()
            .expect("The static Cell should always be initialized at the start of the program")
    }
}

// SAFETY: uefi is a single threaded environment there is no notion of thread safety
unsafe impl Sync for SecurityOverride {}

/// A guard for [`SecurityOverride`]. When created, it will install a validator. When the
/// override is eventually dropped, the validator will be uninstalled.
pub(super) struct SecurityOverrideGuard;

impl SecurityOverrideGuard {
    /// Create a new [`SecurityOverrideGuard`]. Installs a validator and returns the guard.
    ///
    /// When the returned guard is dropped, the security override is automatically uninstalled.
    pub(super) fn new(validator: Validator, validator_ctx: Option<NonNull<u8>>) -> Self {
        install_security_override(validator, validator_ctx);
        Self
    }
}

impl Drop for SecurityOverrideGuard {
    fn drop(&mut self) {
        uninstall_security_override();
    }
}

/// Tests if secure boot is enabled through a UEFI variable.
#[must_use = "Has no effect if the result is unused"]
fn secure_boot_enabled() -> bool {
    matches!(
        get_variable::<u8>(cstr16!("SecureBoot"), Some(VariableVendor::GLOBAL_VARIABLE)),
        Ok(1)
    )
}

/// Installs a security override given a [`Validator`] and optionally a `validator_ctx`.
///
/// You should use the [`SecurityOverrideGuard`] to safely ensure the override is dropped.
fn install_security_override(validator: Validator, validator_ctx: Option<NonNull<u8>>) {
    let security_override = &SECURITY_OVERRIDE;

    security_override
        .inner
        .set(Some(SecurityOverrideInner::new(validator, validator_ctx)));
}

/// Uninstalls the security override. Should be used after installing the security override.
///
/// You should use the [`SecurityOverrideGuard`] to safely ensure the override is dropped.
fn uninstall_security_override() {
    let security_override = &SECURITY_OVERRIDE;

    security_override.get().uninstall_validator();
    security_override.inner.take();
}

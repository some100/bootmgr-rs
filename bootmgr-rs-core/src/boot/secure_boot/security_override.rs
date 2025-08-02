//! Provide [`SecurityOverrideInner`], which is what handles validation with custom hooks.
//!
//! This is most applicable for usage with Shim, as before Shim v16, validation must be manually done using the `ShimLock` protocol.
//! This adopts an approach very, very similar to systemd-boot's security override installation. Essentially, the methods
//! `FileAuthenticationState` and `FileAuthentication` are hijacked from whichever [`Handle`] implements those methods, and replaced
//! with our own. Because the firmware calls upon these methods for validation, this allows us to replace the firmware's secure boot with
//! Shim's validator or another validator of our choice.

use core::{ffi::c_void, ptr::NonNull};

use uefi::{
    Handle, Status,
    proto::device_path::{DevicePath, FfiDevicePath},
};

use crate::{
    BootResult,
    boot::secure_boot::{SecureBootError, Validator, secure_boot_enabled},
    system::protos::{Security2ArchProtocol, SecurityArchProtocol},
};

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

/// The main handler for the security override
#[derive(Clone, Copy, Default)]
pub struct SecurityOverrideInner {
    /// The [`Handle`] that supports [`SecurityArchProtocol`].
    pub security: Option<Handle>,

    /// The [`Handle`] that supports [`Security2ArchProtocol`].
    pub security2: Option<Handle>,

    /// The original method for [`SecurityArchProtocol`] that was used in `LoadImage` before the override.
    pub original_hook: Option<AuthState>,

    /// The original method for [`Security2ArchProtocol`] that was used in `LoadImage` before the override.
    pub original_hook2: Option<Authentication>,

    /// The custom validator installed.
    pub validator: Option<Validator>,

    /// The context for the validator if required.
    pub validator_ctx: Option<NonNull<u8>>,
}

impl SecurityOverrideInner {
    /// Installs a custom validator.
    ///
    /// This validator must be of type [`Validator`], and may optionally have a persistent `validator_ctx` state.
    /// This context is a `NonNull<u8>` and should be cast to and from whatever type you're using as context.
    ///
    /// # Example
    /// ```
    /// fn some_validate(
    ///     _ctx: Option<core::ptr::NonNull<u8>>,
    ///     _device_path: Option<&uefi::proto::device_path::DevicePath>,
    ///     _file_buffer: Option<&mut [u8]>,
    ///     _file_size: usize,
    /// ) -> BootResult<()> {
    ///     Ok(()) // a validator that accepts every image
    /// }
    ///
    /// use bootmgr_rs_core::BootResult;
    /// use bootmgr_rs_core::boot::secure_boot::security_override::SecurityOverrideInner;
    ///
    /// let security_override = SecurityOverrideInner::default();
    ///
    /// security_override.install_validator(some_validate, None);
    /// ```
    pub fn install_validator(&mut self, validator: Validator, validator_ctx: Option<NonNull<u8>>) {
        if self.should_skip_install(validator, validator_ctx) {
            return;
        }

        self.install_security1_hook();
        self.install_security2_hook();

        self.validator = Some(validator);
        self.validator_ctx = validator_ctx;
    }

    /// Uninstalls the custom validator.
    ///
    /// Note that this method takes `&self`, which means that it does not modify any of the inner members.
    /// It only uninstalls the security hooks from the [`SecurityArchProtocol`] and [`Security2ArchProtocol`]
    /// handles, which should be enough.
    pub fn uninstall_validator(&self) {
        self.uninstall_security1_hook();
        self.uninstall_security2_hook();
    }

    /// Checks if the security override should not be installed.
    ///
    /// If the validators are exactly the same (function pointer addresses are equal), or secure boot
    /// is not enabled, then it returns [`false`].
    fn should_skip_install(
        &mut self,
        validator: Validator,
        validator_ctx: Option<NonNull<u8>>,
    ) -> bool {
        if let Some(security_validator) = self.validator {
            if core::ptr::fn_addr_eq(validator, security_validator)
                && self.validator_ctx == validator_ctx
            {
                // if the two validators are equal, there is nothing new to install
                return true;
            }
            self.uninstall_validator();
        }

        if !secure_boot_enabled() {
            return true;
        }

        false
    }

    /// Calls the validator that was installed onto the security protocols.
    ///
    /// # Errors
    ///
    /// May return an `Error` if there is no validator, or the validator deems the image as having failed.
    pub fn call_validator(
        &self,
        device_path: Option<&DevicePath>,
        file_buffer: Option<&mut [u8]>,
    ) -> BootResult<()> {
        if let Some(validator) = self.validator {
            let validator_ctx = self.validator_ctx;

            let file_size = match file_buffer {
                Some(ref file_buffer) => file_buffer.len(),
                None => 0,
            };

            validator(validator_ctx, device_path, file_buffer, file_size)
        } else {
            Err(SecureBootError::NoValidator.into())
        }
    }

    /// Calls the original hook for [`SecurityArchProtocol`] that was there previously before the custom validator was installed.
    ///
    /// This should only be called in the security hook function. You should never have to use this directly.
    ///
    /// # Safety
    ///
    /// This function takes raw pointers as parameters, which means that if null or misaligned pointers are
    /// passed to this function, those will be dereferenced, which is UB.
    ///
    /// The caller must ensure that the pointers passed to this function are not invalid pointers.
    pub unsafe fn call_original_hook(
        &self,
        this: *const SecurityArchProtocol,
        auth_status: u32,
        file: *const FfiDevicePath,
    ) -> Status {
        match self.original_hook {
            Some(original_hook) => unsafe { original_hook(this, auth_status, file) },
            None => Status::SUCCESS,
        }
    }

    /// Calls the original hook for [`Security2ArchProtocol`] that was there previously before the custom validator was installed.
    ///
    /// This should only be called in the security hook function. You should never have to use this directly.
    ///
    /// # Safety
    ///
    /// This function takes raw pointers as parameters, which means that if null or misaligned pointers are
    /// passed to this function, those will be dereferenced, which is UB.
    ///
    /// The caller must ensure that the pointers passed to this function are not invalid pointers.
    pub unsafe fn call_original_hook2(
        &self,
        this: *const Security2ArchProtocol,
        device_path: *const FfiDevicePath,
        file_buffer: *mut c_void,
        file_size: usize,
        boot_policy: u8,
    ) -> Status {
        match self.original_hook2 {
            Some(original_hook2) => unsafe {
                original_hook2(this, device_path, file_buffer, file_size, boot_policy)
            },
            None => Status::SUCCESS,
        }
    }
}

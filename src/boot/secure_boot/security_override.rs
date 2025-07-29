//! Provide [`SecurityOverrideInner`], which is what handles validation with custom hooks.
//!
//! This is most applicable for usage with Shim, as before Shim v16, validation must be manually done using the `ShimLock` protocol.
//! This adopts an approach very, very similar to systemd-boot's security override installation. Essentially, the methods
//! `FileAuthenticationState` and `FileAuthentication` are hijacked from whichever [`Handle`] implements those methods, and replaced
//! with our own. Because the firmware calls upon these methods for validation, this allows us to replace the firmware's secure boot with
//! Shim's validator.

use core::{ffi::c_void, ptr::NonNull};

use uefi::{
    Handle, Status, boot,
    proto::device_path::{DevicePath, FfiDevicePath},
};

use crate::{
    BootResult,
    boot::secure_boot::{
        AuthState, Authentication, SecureBootError, Validator, secure_boot_enabled, security_hook,
        security2_hook,
    },
    system::protos::{Security2Arch, Security2ArchProtocol, SecurityArch, SecurityArchProtocol},
};

/// The main handler for the security override
#[derive(Clone, Copy, Default)]
pub struct SecurityOverrideInner {
    /// The [`Handle`] that supports [`SecurityArch`].
    pub security: Option<Handle>,

    /// The [`Handle`] that supports [`Security2Arch`].
    pub security2: Option<Handle>,

    /// The original method for [`SecurityArch`] that was used in `LoadImage` before the override.
    pub original_hook: Option<AuthState>,

    /// The original method for [`Security2Arch`] that was used in `LoadImage` before the override.
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
    /// In the `bootmgr-rs` application, the validator may be set a grand total of one time due to it using
    /// a `OnceCell` behind a static variable. This simply uninstalls the validator from the firmware's
    /// security hooks.
    pub fn uninstall_validator(&self) {
        self.uninstall_security1_hook();
        self.uninstall_security2_hook();
    }

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

    fn install_security1_hook(&mut self) {
        if let Ok(handle) = boot::get_handle_for_protocol::<SecurityArch>()
            && let Ok(mut security) = boot::open_protocol_exclusive::<SecurityArch>(handle)
        {
            security.get_inner_mut().auth_state = security_hook;
            self.original_hook = Some(security.get_inner().auth_state);
            self.security = Some(handle);
        }
    }

    fn install_security2_hook(&mut self) {
        if let Ok(handle) = boot::get_handle_for_protocol::<Security2Arch>()
            && let Ok(mut security) = boot::open_protocol_exclusive::<Security2Arch>(handle)
        {
            security.get_inner_mut().authentication = security2_hook;
            self.original_hook2 = Some(security.get_inner().authentication);
            self.security2 = Some(handle);
        }
    }

    fn uninstall_security1_hook(&self) {
        if let Some(original_hook) = self.original_hook
            && let Some(handle) = self.security
            && let Ok(mut security) = boot::open_protocol_exclusive::<SecurityArch>(handle)
        {
            security.get_inner_mut().auth_state = original_hook;
        }
    }

    fn uninstall_security2_hook(&self) {
        if let Some(original_hook2) = self.original_hook2
            && let Some(handle) = self.security2
            && let Ok(mut security) = boot::open_protocol_exclusive::<Security2Arch>(handle)
        {
            security.get_inner_mut().authentication = original_hook2;
        }
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

    /// Calls the original hook for [`SecurityArch`] that was there previously before the custom validator was installed.
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

    /// Calls the original hook for [`Security2Arch`] that was there previously before the custom validator was installed.
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

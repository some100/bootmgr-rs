//! Secure Boot support module.

use uefi::{Status, boot, cstr16, runtime::VariableVendor};

use crate::system::{protos::Security2Arch, variable::get_variable};

/// Verifies an image with the firmware's Secure Boot support.
///
/// It's worth noting that this may be considered technically redundant as any security violation errors
/// will be caught by `LoadImage`. However, we do catch this specific error a little bit earlier this way
/// (though not that much earlier).
///
/// # Errors
///
/// May return an `Error` if the firmware does not support [`Security2Arch`], or the image could not be loaded
/// due to a security violation.
pub fn verify_image(file_buffer: &mut [u8]) -> uefi::Result<()> {
    if secure_boot_enabled() {
        let handle = boot::get_handle_for_protocol::<Security2Arch>()?;
        let security_arch = boot::open_protocol_exclusive::<Security2Arch>(handle)?;

        match security_arch.authentication(None, file_buffer, false) {
            Status::ACCESS_DENIED | // for error handling purposes they're basically the same
            Status::SECURITY_VIOLATION => return Err(Status::SECURITY_VIOLATION.into()),
            _ => (),
        }
    }

    Ok(())
}

/// Tests if secure boot is enabled through a UEFI variable.
#[must_use]
pub fn secure_boot_enabled() -> bool {
    if let Ok(var) = get_variable::<u8>(cstr16!("SecureBoot"), Some(VariableVendor::GLOBAL_VARIABLE))
        && var == 1
    {
        return true;
    }
    false
}

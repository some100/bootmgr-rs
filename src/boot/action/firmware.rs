//! Provides [`reset_to_firmware`] which allows to reboot to firmware setup

use uefi::{
    Status, cstr16,
    runtime::{self, ResetType, VariableAttributes, VariableVendor},
};

use crate::{
    BootResult,
    error::BootError,
    system::variable::{get_variable, set_variable},
};

/// The bit that indicates to the firmware if booting into firmware setup should be done.
pub const EFI_OS_INDICATIONS_BOOT_TO_FW_UI: u64 = 1;

/// Reboots to firmware setup using the `OsIndications` variable
///
/// Gets the `OsIndications` variable, optionally creates it if it does not already exists, then
/// sets the [`EFI_OS_INDICATIONS_BOOT_TO_FW_UI`] bit indicating to the firmware to reboot into the
/// setup.
///
/// # Errors
///
/// May return an `Error` for many reasons, see [`uefi::runtime::get_variable`] and [`uefi::runtime::set_variable`]
pub fn reset_to_firmware() -> BootResult<!> {
    if !is_supported()? {
        return Err(BootError::Uefi(Status::UNSUPPORTED.into()));
    }
    set_reset_to_firmware_flag()?;
    runtime::reset(ResetType::WARM, Status::SUCCESS, None); // never returns, ever, and cannot fail
}

// Sets the EFI_OS_INDICATIONS_BOOT_TO_FW_UI bit.
fn set_reset_to_firmware_flag() -> BootResult<()> {
    let mut osind = get_variable::<u64>(
        cstr16!("OsIndications"),
        Some(VariableVendor::GLOBAL_VARIABLE),
    )?;
    osind |= EFI_OS_INDICATIONS_BOOT_TO_FW_UI;
    set_variable::<u64>(
        cstr16!("OsIndications"),
        Some(VariableVendor::GLOBAL_VARIABLE),
        Some(
            VariableAttributes::NON_VOLATILE
                | VariableAttributes::BOOTSERVICE_ACCESS
                | VariableAttributes::RUNTIME_ACCESS,
        ),
        Some(osind),
    )?;
    Ok(())
}

// Checks rebooting to firmware is supported.
fn is_supported() -> BootResult<bool> {
    let supported = get_variable::<u64>(
        cstr16!("OsIndicationsSupported"),
        Some(VariableVendor::GLOBAL_VARIABLE),
    )?;
    Ok(supported & EFI_OS_INDICATIONS_BOOT_TO_FW_UI > 0)
}

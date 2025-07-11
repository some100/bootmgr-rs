//! Provides [`reset_to_firmware`] which allows to reboot to firmware setup

use uefi::{
    Status, cstr16,
    runtime::{self, ResetType, VariableAttributes, VariableVendor},
};

use crate::system::variable::{get_variable, set_variable};

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
pub fn reset_to_firmware() -> uefi::Result<()> {
    if !is_supported()? {
        return Err(Status::UNSUPPORTED.into());
    }
    set_reset_to_firmware_flag()?;
    runtime::reset(ResetType::WARM, Status::SUCCESS, None); // never returns, ever
}

// Sets the EFI_OS_INDICATIONS_BOOT_TO_FW_UI bit.
fn set_reset_to_firmware_flag() -> uefi::Result<()> {
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
        osind,
    )?;
    Ok(())
}

// Checks if it is supported to reboot to firmware
fn is_supported() -> uefi::Result<bool> {
    let supported = get_variable::<u64>(
        cstr16!("OsIndicationsSupported"),
        Some(VariableVendor::GLOBAL_VARIABLE),
    )?;
    Ok(supported & EFI_OS_INDICATIONS_BOOT_TO_FW_UI > 0)
}

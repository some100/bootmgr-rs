//! Provides [`reset_to_firmware`] which allows to reboot to firmware setup

use log::error;
use uefi::{
    Status, boot, cstr16,
    runtime::{self, ResetType, VariableAttributes, VariableVendor},
};

use crate::{
    BootResult,
    system::variable::{get_variable, set_variable},
};

/// The bit that indicates to the firmware if booting into firmware setup should be done.
const EFI_OS_INDICATIONS_BOOT_TO_FW_UI: u64 = 1;

/// Reboots to firmware setup using the `OsIndications` variable
///
/// Gets the `OsIndications` variable, optionally creates it if it does not already exists, then
/// sets the `EFI_OS_INDICATIONS_BOOT_TO_FW_UI` bit indicating to the firmware to reboot into the
/// setup.
///
/// If the `OsIndications` could not be set for some reason, the error will be displayed on screen for 5
/// seconds, then the system will reboot. This is because this function never returns, so control cannot be
/// returned to the main loop.
pub fn reset_to_firmware() -> ! {
    if let Err(e) = set_reset_to_firmware_flag() {
        error!("Failed to set OsIndications: {e}");
        boot::stall(5_000_000);
    }
    runtime::reset(ResetType::WARM, Status::SUCCESS, None)
}

/// Sets the [`EFI_OS_INDICATIONS_BOOT_TO_FW_UI`] bit.
fn set_reset_to_firmware_flag() -> BootResult<()> {
    let mut osind = get_variable::<u64>(
        cstr16!("OsIndications"),
        Some(VariableVendor::GLOBAL_VARIABLE),
    )?; // returns 0 on not found, so if the variable does not exist, then it will not error
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

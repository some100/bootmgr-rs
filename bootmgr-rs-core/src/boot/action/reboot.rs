//! Provides [`reset`] which reboots the system.

use uefi::{
    Status,
    runtime::{self, ResetType},
};

/// Resets the system.
///
/// This function wraps around [`runtime::reset`] and provides a slightly more straightforward way to reboot the system.
pub fn reset() -> ! {
    runtime::reset(ResetType::WARM, Status::SUCCESS, None) // never returns, ever, and cannot fail
}

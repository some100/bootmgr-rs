//! Provides [`shutdown`] which shuts down the system.

use uefi::{
    Status,
    runtime::{self, ResetType},
};

/// Shuts down the system.
///
/// This function wraps around [`runtime::reset`] and provides a slightly more straightforward way to shutdown the system.
pub fn shutdown() -> ! {
    runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None) // never returns, ever, and cannot fail
}

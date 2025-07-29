//! Simple panic handler that stalls for 10 seconds, then shuts down the system.
//!
//! This is disabled when fuzzing or testing, to avoid conflicts with std.

#![cfg(all(not(fuzzing), not(test)))] // this causes a conflict if fuzzing or testing if enabled
use core::fmt::Write;

const PANIC_DELAY: usize = 10_000_000; // 10 seconds

#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    uefi::system::with_stdout(|stdout| {
        let _ = writeln!(stdout, "[PANIC]: {info}");
    });
    uefi::boot::stall(PANIC_DELAY);
    uefi::runtime::reset(
        uefi::runtime::ResetType::SHUTDOWN,
        uefi::Status::ABORTED,
        None,
    );
}

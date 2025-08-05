//! Simple panic handler that stalls for 10 seconds, then shuts down the system.
//!
//! This is enabled when the `panic_handler` feature is enabled, in case the user wanted to roll their
//! own panic handler implementation or for fuzzing/testing.

#![cfg(feature = "panic_handler")]
use core::fmt::Write;

/// The panic handler.
#[panic_handler]
fn panic_handler(info: &core::panic::PanicInfo) -> ! {
    uefi::system::with_stdout(|stdout| {
        let _ = writeln!(stdout, "[PANIC]: {info}");
        let _ = writeln!(stdout, "Press a key to shut down");
    });
    uefi::system::with_stdin(|stdin| {
        if let Some(event) = stdin.wait_for_key_event() {
            let _ = uefi::boot::wait_for_event(&mut [event]);
        } else {
            uefi::system::with_stdout(|stdout| {
                let _ = writeln!(
                    stdout,
                    "Failed to get key event, shutting down in 10 seconds"
                );
            });
            uefi::boot::stall(10_000_000);
        }
    });
    uefi::runtime::reset(
        uefi::runtime::ResetType::SHUTDOWN,
        uefi::Status::ABORTED,
        None,
    );
}

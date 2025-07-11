#![cfg(all(not(fuzzing), not(test)))] // this causes a conflict if fuzzing or testing if enabled
use core::fmt::Write;

const PANIC_DELAY: usize = 10_000_000;

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

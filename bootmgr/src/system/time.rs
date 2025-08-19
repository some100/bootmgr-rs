// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Time measuring module.
//!
//! # Safety
//!
//! This uses unsafe in 4 places, though only 2 at most are enabled per platform.
//!
//! 1. `_rdtsc` is not a serializing instruction, which is why it is marked unsafe. However, this problem does not exist
//!    in UEFI as it is a completely single threaded environment. Therefore, it is safe.
//! 2. See point 1.
//! 3. Inline assembly is practically always unsafe, however this specific segment is safe as it only reads from `CNTVCT_EL0`,
//!    which is the counter of the timer.
//! 4. See point 3, but replace `CNTVCT_EL0` with `CNTFRQ_EL0` and "counter" with "frequency".

use core::cell::LazyCell;

/// The frequency of the timer, stored statically in a variable for efficiency.
///
/// This is done so that the potentially expensive [`timer_freq`] operation (depending on x86 or aarch64) is only done
/// once when it is used.
static TIMER_FREQ: TimerFreq = TimerFreq {
    timer_freq: LazyCell::new(timer_freq),
};

/// A timer frequency that is stored in a static variable.
struct TimerFreq {
    /// The frequency of the timer, initialized once at the beginning using a [`LazyCell`].
    timer_freq: LazyCell<u64>,
}

// SAFETY: UEFI is single threaded there is no requirement of thread safety.
unsafe impl Sync for TimerFreq {}

/// Read the value of the system's timestamp counter, or timer tick.
#[must_use = "Has no effect if the result is unused"]
fn timer_tick() -> u64 {
    // SAFETY: this simply reads the current value of the tsc. this should be safe, since this only calls one reasonably safe instruction.
    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::x86::_rdtsc()
    }

    // SAFETY: this simply reads the current value of the tsc. this should be safe, since this only calls one reasonably safe instruction.
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }

    // SAFETY: this simply reads the current value of cntvct_el0. this should be safe, as we only do this to read the timer counter and nothing more.
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut ticks: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
        ticks
    }
}

/// Get the frequency of timer ticks on this system.
#[must_use = "Has no effect if the result is unused"]
fn timer_freq() -> u64 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let start = timer_tick();
        uefi::boot::stall(1000);
        let end = timer_tick();
        (end - start) * 1000
    }

    // SAFETY: this simply reads the current value of cntfrq_el0. this should be safe, as we only do this to read the timer freq and nothing more.
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut freq: u64;
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq);
        freq
    }
}

/// Get the number of microseconds since system initialization.
#[must_use = "Has no effect if the result is unused"]
pub fn timer_usec() -> u64 {
    1000 * 1000 * timer_tick() / *TIMER_FREQ.timer_freq
}

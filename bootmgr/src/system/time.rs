// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Time measuring module.
//!
//! # Safety
//!
//! This uses unsafe in 4 places, though only 2 at most are enabled per platform.
//!
//! 1. For x86 and x64, `_rdtsc` is not a serializing instruction, which is why it is marked unsafe. However, this problem
//!    does not exist in UEFI as it is a completely single threaded environment. Therefore, it is safe. For aarch64, only
//!    `CNTVCT_EL0` is read, which is the counter of the timer. This is safe, because it doesn't write to any registers.
//! 2. Inline assembly is practically always unsafe, however this specific segment is safe as it only reads from `CNTFRQ_EL0`,
//!    which is the frequency of the timer.

use core::{cell::LazyCell, time::Duration};

/// The frequency of the timer, stored statically in a variable for efficiency.
///
/// This is done so that the potentially expensive [`timer_freq`] operation (depending on x86 or aarch64) is only done
/// once when it is used.
static TIMER_FREQ: TimerFreq = TimerFreq(LazyCell::new(timer_freq));

/// A timer frequency that is stored in a static variable.
struct TimerFreq(LazyCell<u64>);

// SAFETY: UEFI is single threaded there is no requirement of thread safety.
unsafe impl Sync for TimerFreq {}

/// A set moment in time. Usually used for comparing with another Instant or in a Duration.
#[derive(Clone, Copy, Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct Instant(u64);

impl Instant {
    /// Returns an `Instant` corresponding to “now”.
    #[must_use = "Has no effect if the result is unused"]
    pub fn now() -> Self {
        Self(1000 * 1000 * timer_tick() / *TIMER_FREQ.0)
    }

    /// Returns an `Instant` corresponding to zero.
    #[must_use = "Has no effect if the result is unused"]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns the amount of time elapsed from another `Instant` to this one. This will return 0 if
    /// that `Instant` was later than the current one.
    #[must_use = "Has no effect if the result is unused"]
    pub const fn duration_since(&self, earlier: Self) -> Duration {
        Duration::from_micros(self.0.saturating_sub(earlier.0))
    }

    /// Get the duration elapsed since this `Instant`.
    #[must_use = "Has no effect if the result is unused"]
    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }
}

/// Read the value of the system's timestamp counter, or timer tick.
#[must_use = "Has no effect if the result is unused"]
fn timer_tick() -> u64 {
    // SAFETY: these only read from a hardware counter, which is safe since UEFI is single threaded
    unsafe {
        #[cfg(target_arch = "x86")]
        {
            core::arch::x86::_rdtsc()
        }

        #[cfg(target_arch = "x86_64")]
        {
            core::arch::x86_64::_rdtsc()
        }

        #[cfg(target_arch = "aarch64")]
        {
            let mut ticks: u64;
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) ticks);
            ticks
        }
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

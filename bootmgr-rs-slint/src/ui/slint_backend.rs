// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: GPL-3.0-or-later

//! The user interface backend for Slint.
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

use core::time::Duration;

use alloc::{boxed::Box, rc::Rc};
use bytemuck::TransparentWrapper;
use slint::{
    Color as SlintColor,
    platform::{
        Platform, WindowAdapter,
        software_renderer::{
            MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
        },
    },
};
use uefi::proto::console::{gop::BltPixel, text::Color as UefiColor};

use crate::{MainError, ui::slint_inc::Ui};

/// A thin wrapper around [`BltPixel`] that implements [`TargetPixel`].
#[repr(transparent)]
#[derive(Clone, Copy, TransparentWrapper)]
pub struct SlintBltPixel(BltPixel);

impl SlintBltPixel {
    /// Create a new black [`SlintBltPixel`].
    pub const fn new() -> Self {
        Self(BltPixel::new(0, 0, 0))
    }
}

impl TargetPixel for SlintBltPixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let a = u16::from(u8::MAX - color.alpha);
        self.0.red = u8::try_from(u16::from(self.0.red) * a / 255).unwrap_or(0) + color.red;
        self.0.green = u8::try_from(u16::from(self.0.green) * a / 255).unwrap_or(0) + color.green;
        self.0.blue = u8::try_from(u16::from(self.0.blue) * a / 255).unwrap_or(0) + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(BltPixel::new(red, green, blue))
    }
}

/// The UEFI backend for Slint.
pub struct UefiPlatform {
    /// An instance of [`MinimalSoftwareWindow`], which renders with the software renderer.
    window: Rc<MinimalSoftwareWindow>,

    /// The frequency of timer "ticks".
    timer_freq: f64,

    /// The value of the timer at the start of the program.
    timer_start: f64,
}

impl Platform for UefiPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        Duration::from_secs_f64(
            (lossy_u64_to_f64(timer_tick()) - self.timer_start) / self.timer_freq,
        )
    }

    // run_event_loop intentionally not implemented
}

/// Read the value of the system's timestamp counter, or timer tick.
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

/// Create a slint window.
pub fn create_window() -> Result<(Rc<MinimalSoftwareWindow>, Ui), MainError> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::default());
    let _ = slint::platform::set_platform(Box::new(UefiPlatform {
        window: window.clone(),
        timer_freq: lossy_u64_to_f64(timer_freq()),
        timer_start: lossy_u64_to_f64(timer_tick()),
    }));

    let ui = Ui::new().map_err(MainError::SlintError)?;

    Ok((window, ui))
}

/// Converts a UEFI color to a Slint color.
pub const fn ueficolor_to_slintcolor(color: UefiColor) -> SlintColor {
    match color {
        UefiColor::Black => SlintColor::from_rgb_u8(0, 0, 0),
        UefiColor::Blue => SlintColor::from_rgb_u8(0, 0, 255),
        UefiColor::Green => SlintColor::from_rgb_u8(0, 255, 0),
        UefiColor::Cyan => SlintColor::from_rgb_u8(0, 255, 255),
        UefiColor::Red => SlintColor::from_rgb_u8(255, 0, 0),
        UefiColor::Magenta => SlintColor::from_rgb_u8(255, 0, 255),
        UefiColor::Brown => SlintColor::from_rgb_u8(150, 75, 0),
        UefiColor::LightGray => SlintColor::from_rgb_u8(211, 211, 211),
        UefiColor::DarkGray => SlintColor::from_rgb_u8(169, 169, 169),
        UefiColor::LightBlue => SlintColor::from_rgb_u8(173, 216, 230),
        UefiColor::LightGreen => SlintColor::from_rgb_u8(144, 238, 144),
        UefiColor::LightCyan => SlintColor::from_rgb_u8(224, 255, 255),
        UefiColor::LightRed => SlintColor::from_rgb_u8(238, 36, 0),
        UefiColor::LightMagenta => SlintColor::from_rgb_u8(255, 128, 255),
        UefiColor::Yellow => SlintColor::from_rgb_u8(255, 255, 0),
        UefiColor::White => SlintColor::from_rgb_u8(255, 255, 255),
    }
}

/// Convert a `u64` into an `f64`, with the possibility of precision loss when casting.
#[allow(
    clippy::cast_precision_loss,
    reason = "f64 is exactly precise up to 2^53, which is more than enough"
)]
const fn lossy_u64_to_f64(num: u64) -> f64 {
    num as f64
}

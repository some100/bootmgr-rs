// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! The user interface backend for Slint.

use core::time::Duration;

use alloc::{boxed::Box, rc::Rc};
use bootmgr::system::time::Instant;
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
        self.0.red = u8::try_from(u16::from(self.0.red) * a / 255).unwrap_or(255) + color.red;
        self.0.green = u8::try_from(u16::from(self.0.green) * a / 255).unwrap_or(255) + color.green;
        self.0.blue = u8::try_from(u16::from(self.0.blue) * a / 255).unwrap_or(255) + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Self(BltPixel::new(red, green, blue))
    }
}

/// The UEFI backend for Slint.
pub struct UefiPlatform {
    /// An instance of [`MinimalSoftwareWindow`], which renders with the software renderer.
    window: Rc<MinimalSoftwareWindow>,

    /// The value of the timer at the start of the program.
    timer_start: Instant,
}

impl Platform for UefiPlatform {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> Duration {
        Instant::now().duration_since(self.timer_start)
    }

    // run_event_loop intentionally not implemented
}

/// Create a slint window.
///
/// # Errors
///
/// May return an `Error` if the `Ui` could not be created.
pub fn create_window() -> Result<(Rc<MinimalSoftwareWindow>, Ui), MainError> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::default());
    let _ = slint::platform::set_platform(Box::new(UefiPlatform {
        window: window.clone(),
        timer_start: Instant::now(),
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

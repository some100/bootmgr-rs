// for some reason, slint auto generated files count towards warns for this lint as well
#![allow(clippy::missing_docs_in_private_items)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]

use core::time::Duration;

use alloc::{borrow::ToOwned, boxed::Box, format, rc::Rc, vec};
use slint::{
    Color as SlintColor, PhysicalSize, PlatformError, SharedString,
    platform::{
        Key as SlintKey, Platform, WindowAdapter, WindowEvent,
        software_renderer::{
            MinimalSoftwareWindow, PremultipliedRgbaColor, RepaintBufferType, TargetPixel,
        },
    },
};
use uefi::{
    boot,
    proto::console::{
        gop::{BltOp, BltPixel, BltRegion, GraphicsOutput},
        text::{Color as UefiColor, Input, Key as UefiKey, ScanCode},
    },
};

use crate::MainError;

slint::include_modules!();

/// A thin wrapper around [`BltPixel`] that implements [`TargetPixel`].
#[repr(transparent)]
#[derive(Clone, Copy)]
struct SlintBltPixel(BltPixel);

impl TargetPixel for SlintBltPixel {
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        let a = u16::from(u8::MAX - color.alpha);
        self.0.red = (u16::from(self.0.red) * a / 255) as u8 + color.red;
        self.0.green = (u16::from(self.0.green) * a / 255) as u8 + color.green;
        self.0.blue = (u16::from(self.0.blue) * a / 255) as u8 + color.blue;
    }

    fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        SlintBltPixel(BltPixel::new(red, green, blue))
    }
}

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
        Duration::from_secs_f64((timer_tick() as f64 - self.timer_start) / self.timer_freq)
    }

    /// Run the event loop.
    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        // this is about the best we can do in terms of error handling, as it is slint::PlatformError.
        let handle = boot::get_handle_for_protocol::<GraphicsOutput>()
            .map_err(|_| PlatformError::Other("No graphics output found on system".to_owned()))?;

        let mut gop = boot::open_protocol_exclusive::<GraphicsOutput>(handle)
            .map_err(|e| PlatformError::Other(format!("Failed to initialize GOP: {e}")))?;

        let res = gop.current_mode_info().resolution();
        let (w, h) = res;
        let mut fb = vec![SlintBltPixel(BltPixel::new(0, 0, 0)); w * h];

        self.window.set_size(PhysicalSize::new(w as u32, h as u32));

        loop {
            slint::platform::update_timers_and_animations();

            if let Some(key) = read_key() {
                let str = SharedString::from(key);
                self.window
                    .try_dispatch_event(WindowEvent::KeyPressed { text: str.clone() })?;
                self.window
                    .try_dispatch_event(WindowEvent::KeyReleased { text: str })?;
            }

            self.window.draw_if_needed(|renderer| {
                renderer.render(&mut fb, w);

                // SAFETY: we just allocated fb on the heap a little bit ago, so this should be safe.
                // also, as a vec, fb.len() should always be accurate to the actual size, so it is also safe.
                // fb is also made of SlintBltPixel, which is simply a repr(transparent) wrapper around BltPixel,
                // so they can be safely casted.
                let blt_fb = unsafe {
                    core::slice::from_raw_parts(fb.as_ptr().cast::<BltPixel>(), fb.len())
                };

                let _ = gop.blt(BltOp::BufferToVideo {
                    buffer: blt_fb,
                    src: BltRegion::Full,
                    dest: (0, 0),
                    dims: res,
                });
            });
        }
    }
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
        timer_freq: timer_freq() as f64,
        timer_start: timer_tick() as f64,
    }));

    let ui = Ui::new().map_err(MainError::SlintError)?;

    Ok((window, ui))
}

/// Read a key from the input.
fn read_key() -> Option<char> {
    let handle = boot::get_handle_for_protocol::<Input>().ok()?;
    let mut input = boot::open_protocol_exclusive::<Input>(handle).ok()?;
    match input.read_key() {
        Ok(Some(UefiKey::Printable(char))) if char == '\r' => Some('\n'),
        Ok(Some(UefiKey::Printable(char))) => Some(char::from(char)),
        Ok(Some(UefiKey::Special(char))) => Some(
            match char {
                ScanCode::LEFT => SlintKey::LeftArrow,
                ScanCode::RIGHT => SlintKey::RightArrow,
                ScanCode::ESCAPE => SlintKey::Escape,
                _ => return None,
            }
            .into(),
        ),
        _ => None,
    }
}

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

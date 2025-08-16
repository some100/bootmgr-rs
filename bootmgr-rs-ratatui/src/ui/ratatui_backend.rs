// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! UEFI Backend for ratatui.

use core::fmt::Write;

use bootmgr_rs_core::{BootResult, error::BootError, system::helper::locate_protocol};
use ratatui_core::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::Color as RatatuiColor,
};
use uefi::{
    Status,
    boot::ScopedProtocol,
    proto::console::text::{Color as UefiColor, Output},
};

/// Convert ANSI colors [`RatatuiColor`] to UEFI foreground colors [`UefiColor`].
///
/// [`RatatuiColor::Reset`], [`RatatuiColor::Rgb`], [`RatatuiColor::Indexed`], and [`RatatuiColor::White`]
/// are mapped to [`UefiColor::White`].
const fn ansi_to_uefi_color_fg(color: RatatuiColor) -> UefiColor {
    match color {
        RatatuiColor::Black => UefiColor::Black,
        RatatuiColor::Red => UefiColor::Red,
        RatatuiColor::Green => UefiColor::Green,
        RatatuiColor::Yellow | RatatuiColor::LightYellow => UefiColor::Yellow,
        RatatuiColor::Blue => UefiColor::Blue,
        RatatuiColor::Magenta => UefiColor::Magenta,
        RatatuiColor::Cyan => UefiColor::Cyan,
        RatatuiColor::Gray => UefiColor::LightGray,
        RatatuiColor::DarkGray => UefiColor::DarkGray,
        RatatuiColor::LightRed => UefiColor::LightRed,
        RatatuiColor::LightGreen => UefiColor::LightGreen,
        RatatuiColor::LightBlue => UefiColor::LightBlue,
        RatatuiColor::LightMagenta => UefiColor::LightMagenta,
        RatatuiColor::LightCyan => UefiColor::LightCyan,
        _ => UefiColor::White, // Reset, Rgb, Indexed, White
    }
}

/// Convert ANSI colors [`RatatuiColor`] to UEFI background colors [`UefiColor`].
///
/// Note that the selection of colors for UEFI backgrounds are much more limited than for foregrounds.
/// Any unrecognized or unusable colors are mapped to [`UefiColor::Black`].
const fn ansi_to_uefi_color_bg(color: RatatuiColor) -> UefiColor {
    // only the first 8 colors may be used for bg
    match color {
        RatatuiColor::Blue => UefiColor::Blue,
        RatatuiColor::Green => UefiColor::Green,
        RatatuiColor::Cyan => UefiColor::Cyan,
        RatatuiColor::Red => UefiColor::Red,
        RatatuiColor::Magenta => UefiColor::Magenta,
        RatatuiColor::Gray => UefiColor::LightGray,
        _ => UefiColor::Black,
    }
}

/// UEFI Backend for ratatui.
pub struct UefiBackend {
    /// The [`Output`] of the UEFI terminal.
    pub output: ScopedProtocol<Output>,

    /// The foreground color.
    pub fg: UefiColor,

    /// The background color.
    pub bg: UefiColor,
}

impl UefiBackend {
    /// Create a new ratatui UEFI backend.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the system does not support an [`Output`].
    pub fn new() -> BootResult<Self> {
        let output = locate_protocol::<Output>()?;
        Ok(Self {
            output,
            fg: UefiColor::White,
            bg: UefiColor::Black,
        })
    }

    /// Set the colors of the terminal.
    pub fn set_color(&mut self, fg: RatatuiColor, bg: RatatuiColor) {
        self.fg = ansi_to_uefi_color_fg(fg);
        self.bg = ansi_to_uefi_color_bg(bg);
        self.reset_color();
    }

    /// Set the colors of the terminal to the ones set previously by [`Self::set_color`].
    pub fn reset_color(&mut self) {
        let _ = self.output.set_color(self.fg, self.bg);
    }
}

impl Backend for UefiBackend {
    type Error = BootError;

    fn draw<'a, I>(&mut self, content: I) -> BootResult<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        for (x, y, cell) in content {
            self.output.set_cursor_position(x as usize, y as usize)?;
            self.output.set_color(
                ansi_to_uefi_color_fg(cell.fg),
                ansi_to_uefi_color_bg(cell.bg),
            )?;

            self.output
                .write_str(cell.symbol())
                .map_err(|_| BootError::Uefi(Status::DEVICE_ERROR.into()))?;
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> BootResult<()> {
        let _ = self.output.enable_cursor(false);
        Ok(())
    }

    fn show_cursor(&mut self) -> BootResult<()> {
        let _ = self.output.enable_cursor(true);
        Ok(())
    }

    fn get_cursor_position(&mut self) -> BootResult<Position> {
        let (x, y) = self.output.cursor_position();

        // as long as your screen has less than 65536 rows and columns,
        // truncation should be generally safe here
        let x = u16::try_from(x).unwrap_or(u16::MAX);
        let y = u16::try_from(y).unwrap_or(u16::MAX);
        Ok((x, y).into())
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> BootResult<()> {
        let Position { x, y } = position.into();
        self.output.set_cursor_position(x as usize, y as usize)?;
        Ok(())
    }

    fn clear(&mut self) -> BootResult<()> {
        self.output.clear()?;
        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> BootResult<()> {
        match clear_type {
            ClearType::All => self.clear(),
            _ => Err(BootError::Uefi(Status::UNSUPPORTED.into())),
        }
    }

    fn size(&self) -> BootResult<Size> {
        let mode = self
            .output
            .current_mode()?
            .ok_or_else(|| BootError::Uefi(Status::UNSUPPORTED.into()))?;
        let columns = u16::try_from(mode.columns()).unwrap_or(u16::MAX);
        let rows = u16::try_from(mode.rows()).unwrap_or(u16::MAX);
        Ok(Size::new(columns, rows))
    }

    fn window_size(&mut self) -> BootResult<WindowSize> {
        Ok(WindowSize {
            columns_rows: self.size()?,
            pixels: Size {
                width: 0,
                height: 0,
            },
        })
    }

    fn flush(&mut self) -> BootResult<()> {
        Ok(())
    }
}

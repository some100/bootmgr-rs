#![allow(clippy::cast_possible_truncation)]
//! UEFI Backend for ratatui

use core::fmt::Write;

use ratatui_core::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::Color as RatatuiColor,
};
use uefi::{
    Status,
    boot::{self, ScopedProtocol},
    proto::console::text::{Color as UefiColor, Output},
};

fn ansi_to_uefi_color_fg(color: RatatuiColor) -> UefiColor {
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

fn ansi_to_uefi_color_bg(color: RatatuiColor) -> UefiColor {
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

pub struct UefiBackend {
    pub output: ScopedProtocol<Output>,
    pub fg: UefiColor,
    pub bg: UefiColor,
}

impl UefiBackend {
    /// Create a new ratatui UEFI backend.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the system does not support an [`Output`].
    pub fn new() -> uefi::Result<Self> {
        let handle = boot::get_handle_for_protocol::<Output>()?;
        let output = boot::open_protocol_exclusive::<Output>(handle)?;
        Ok(Self {
            output,
            fg: UefiColor::White,
            bg: UefiColor::Black,
        })
    }

    #[must_use]
    pub fn with_output(output: ScopedProtocol<Output>) -> Self {
        Self {
            output,
            fg: UefiColor::White,
            bg: UefiColor::Black,
        }
    }

    pub fn set_color(&mut self, fg: RatatuiColor, bg: RatatuiColor) {
        self.fg = ansi_to_uefi_color_fg(fg);
        self.bg = ansi_to_uefi_color_bg(bg);
        self.reset_color();
    }

    pub fn reset_color(&mut self) {
        let _ = self.output.set_color(self.fg, self.bg);
    }
}

impl Backend for UefiBackend {
    type Error = uefi::Error;

    fn draw<'a, I>(&mut self, content: I) -> uefi::Result<()>
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
                .map_err(|_| uefi::Error::new(Status::DEVICE_ERROR, ()))?;
        }
        Ok(())
    }

    fn hide_cursor(&mut self) -> uefi::Result<()> {
        let _ = self.output.enable_cursor(false);
        Ok(())
    }

    fn show_cursor(&mut self) -> uefi::Result<()> {
        let _ = self.output.enable_cursor(true);
        Ok(())
    }

    fn get_cursor_position(&mut self) -> uefi::Result<Position> {
        let (x, y) = self.output.cursor_position();
        Ok((x as u16, y as u16).into())
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> uefi::Result<()> {
        let Position { x, y } = position.into();
        self.output.set_cursor_position(x as usize, y as usize)?;
        Ok(())
    }

    fn clear(&mut self) -> uefi::Result<()> {
        self.output.clear()?;
        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> uefi::Result<()> {
        match clear_type {
            ClearType::All => self.clear(),
            ClearType::AfterCursor
            | ClearType::BeforeCursor
            | ClearType::CurrentLine
            | ClearType::UntilNewLine => Err(uefi::Error::new(Status::UNSUPPORTED, ())),
        }
    }

    fn size(&self) -> uefi::Result<Size> {
        let mode = self
            .output
            .current_mode()?
            .ok_or_else(|| uefi::Error::new(Status::UNSUPPORTED, ()))?;
        Ok(Size::new(mode.columns() as u16, mode.rows() as u16))
    }

    fn window_size(&mut self) -> uefi::Result<WindowSize> {
        let mode = self
            .output
            .current_mode()?
            .ok_or_else(|| uefi::Error::new(Status::UNSUPPORTED, ()))?;
        Ok(WindowSize {
            columns_rows: Size {
                width: mode.columns() as u16,
                height: mode.rows() as u16,
            },
            pixels: Size {
                width: 0,
                height: 0,
            },
        })
    }

    fn flush(&mut self) -> uefi::Result<()> {
        Ok(())
    }
}

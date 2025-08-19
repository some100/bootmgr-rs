// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Provides [`Theme`], which exposes the color scheme for the UI.

use bootmgr::boot::config::BootConfig;
use ratatui_core::style::{Color as RatatuiColor, Style};

use uefi::proto::console::text::Color as UefiColor;

/// The color scheme of the UI.
#[derive(Clone, Copy, Default)]
pub struct Theme {
    /// The color scheme for everything except highlighted items.
    pub base: Style,

    /// The color scheme for highlighted items.
    pub highlight: Style,
}

impl Theme {
    /// Create a new [`Theme`] from a [`BootConfig`].
    #[must_use = "Has no effect if the result is unused"]
    pub const fn new(config: &BootConfig) -> Self {
        Self {
            base: Style::new()
                .fg(uefi_to_ansi_color_fg(config.fg))
                .bg(uefi_to_ansi_color_bg(config.bg)),
            highlight: Style::new()
                .fg(uefi_to_ansi_color_fg(config.highlight_fg))
                .bg(uefi_to_ansi_color_bg(config.highlight_bg)),
        }
    }
}

/// Convert UEFI foreground colors [`UefiColor`] to ANSI colors [`RatatuiColor`].
const fn uefi_to_ansi_color_fg(color: UefiColor) -> RatatuiColor {
    match color {
        UefiColor::Black => RatatuiColor::Black,
        UefiColor::Red => RatatuiColor::Red,
        UefiColor::Green => RatatuiColor::Green,
        UefiColor::Yellow => RatatuiColor::Yellow, // LightYellow also mapped to Yellow originally
        UefiColor::Blue => RatatuiColor::Blue,
        UefiColor::Magenta => RatatuiColor::Magenta,
        UefiColor::Cyan => RatatuiColor::Cyan,
        UefiColor::LightGray => RatatuiColor::Gray,
        UefiColor::DarkGray => RatatuiColor::DarkGray,
        UefiColor::LightRed => RatatuiColor::LightRed,
        UefiColor::LightGreen => RatatuiColor::LightGreen,
        UefiColor::LightBlue => RatatuiColor::LightBlue,
        UefiColor::LightMagenta => RatatuiColor::LightMagenta,
        UefiColor::LightCyan => RatatuiColor::LightCyan,
        _ => RatatuiColor::White,
    }
}

/// Convert UEFI background colors [`UefiColor`] to ANSI colors [`RatatuiColor`].
const fn uefi_to_ansi_color_bg(color: UefiColor) -> RatatuiColor {
    match color {
        UefiColor::Blue => RatatuiColor::Blue,
        UefiColor::Green => RatatuiColor::Green,
        UefiColor::Cyan => RatatuiColor::Cyan,
        UefiColor::Red => RatatuiColor::Red,
        UefiColor::Magenta => RatatuiColor::Magenta,
        UefiColor::LightGray => RatatuiColor::Gray,
        _ => RatatuiColor::Black,
    }
}

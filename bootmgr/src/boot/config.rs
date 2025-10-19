// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Provides [`BootConfig`], the configuration file for the bootloader.
//!
//! This parses space separated key value pairs, the format of which is defined in
//! the [`BootConfig`] struct.
//!
//! The general syntax of the configuration file is not too dissimilar from that of BLS configuration files that
//! come with systemd-boot.
//!
//! Example configuration:
//!
//! ```text
//! # Adjusts the time for the default boot option to be picked
//! timeout 10
//!
//! # Selects the default boot option through its index on the boot list
//! default 3
//!
//! # Change the path where drivers are searched
//! driver_path /EFI/Drivers
//!
//! # Enable or disable the builtin editor provided with the default frontend
//! editor true
//!
//! # Enable or disable PXE boot discovery
//! pxe true
//!
//! # Change the colors of the application
//! bg magenta
//! fg light_yellow
//! highlight_bg gray
//! highlight_fg black
//! ```
//!
//! Frontends are not strictly obligated to honor the theming, default, and timeout settings.
//! They exist as a way to signal user settings to the frontend, and the frontend can choose
//! to implement those settings if needed or possible.
//!
//! Note that colors are stored as UEFI [`Color`]. Therefore, a frontend may need to convert
//! from this color type.

use alloc::{borrow::ToOwned, string::String};

use uefi::{CStr16, Status, cstr16, proto::console::text::Color};

use crate::{
    BootResult,
    system::{
        fs::{FsError, UefiFileSystem},
        helper::normalize_path,
    },
};

/// The hardcoded configuration path for the [`BootConfig`].
const CONFIG_PATH: &CStr16 = cstr16!("\\loader\\bootmgr-rs.conf");

/// The configuration file for the bootloader.
pub struct BootConfig {
    /// The timeout for the bootloader before the default boot option is selected.
    pub timeout: i64,

    /// The default boot option as the index of the entry.
    pub default: Option<usize>,

    /// Whether loading drivers is enabled or not.
    pub drivers: bool,

    /// The path to the drivers in the same filesystem as the bootloader.
    pub driver_path: String,

    /// Allows for the editor to be enabled, if there is one.
    pub editor: bool,

    /// Allows for the basic PXE/TFTP loader to be enabled.
    pub pxe: bool,

    /// Allows adjusting the background of the UI.
    pub bg: Color,

    /// Allows adjusting the foreground of the UI.
    pub fg: Color,

    /// Allows adjusting the background of the highlighter.
    pub highlight_bg: Color,

    /// Allows adjusting the foreground of the highlighter.
    pub highlight_fg: Color,
}

impl BootConfig {
    /// Creates a new [`BootConfig`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the image handle from which this program was loaded from
    /// does not support [`uefi::proto::media::fs::SimpleFileSystem`]. Otherwise, it will
    /// return an empty [`BootConfig`].
    pub(super) fn new() -> BootResult<Self> {
        let mut fs = UefiFileSystem::from_image_fs()?;

        let mut buf = [0; 4096]; // a config file over 4096 bytes is very unusual and is not supported
        let bytes = match fs.read_into(CONFIG_PATH, &mut buf) {
            Ok(bytes) => bytes,
            Err(FsError::OpenErr(Status::NOT_FOUND)) => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };

        Ok(Self::get_boot_config(&buf, Some(bytes)))
    }

    /// Parses the contents of a [`BootConfig`] format string.
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_boot_config(content: &[u8], bytes: Option<usize>) -> Self {
        let mut config = Self::default();
        let slice = &content[0..bytes.unwrap_or(content.len()).min(content.len())];

        #[cfg(not(test))]
        if let Some(timeout) = super::bli::get_timeout_var() {
            config.timeout = timeout;
        }

        if let Ok(content) = str::from_utf8(slice) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                config.assign_to_field(line);
            }
        }

        config
    }

    /// Assign a field to the [`BootConfig`] given a line containing the key and value.
    fn assign_to_field(&mut self, line: &str) {
        if let Some((key, value)) = line.split_once(' ') {
            let value = value.trim().to_owned();
            match &*key.to_ascii_lowercase() {
                "timeout" => {
                    if let Ok(value) = value.parse() {
                        self.timeout = value;

                        #[cfg(not(test))]
                        let _ = super::bli::set_timeout_var(value);
                    }
                }
                "default" => {
                    if let Ok(value) = value.parse() {
                        self.default = Some(value);
                    }
                }
                "drivers" => {
                    if let Ok(value) = value.parse() {
                        self.drivers = value;
                    }
                }
                "driver_path" => {
                    let value = normalize_path(&value);
                    self.driver_path = value;
                }
                "editor" => {
                    if let Ok(value) = value.parse() {
                        self.editor = value;
                    }
                }
                "pxe" => {
                    if let Ok(value) = value.parse() {
                        self.pxe = value;
                    }
                }
                "background" => self.bg = match_str_color_bg(&value),
                "foreground" => self.fg = match_str_color_fg(&value),
                "highlight_background" => self.highlight_bg = match_str_color_bg(&value),
                "highlight_foreground" => self.highlight_fg = match_str_color_fg(&value),
                _ => (),
            }
        }
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            timeout: 5,
            default: None,
            drivers: false,
            driver_path: "\\EFI\\BOOT\\drivers".to_owned(),
            editor: false,
            pxe: false,
            bg: Color::Black,
            fg: Color::White,
            highlight_bg: Color::LightGray,
            highlight_fg: Color::Black,
        }
    }
}

/// Returns a foreground color given a color's string representation.
///
/// Any unrecognized colors will return [`Color::Black`].
fn match_str_color_fg(color: &str) -> Color {
    match color {
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::LightGray,
        "dark_gray" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Black,
    }
}

/// Returns a background color given a color's string representation.
///
/// The pool of colors is significantly less than foreground, and any unrecognized colors
/// will also return [`Color::Black`].
fn match_str_color_bg(color: &str) -> Color {
    match color {
        "blue" => Color::Blue,
        "green" => Color::Green,
        "cyan" => Color::Cyan,
        "red" => Color::Red,
        "magenta" => Color::Magenta,
        "gray" | "white" => Color::LightGray, // close enough
        _ => Color::Black,
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// # Panics
    ///
    /// May panic if the assertions fail.
    #[test]
    fn test_full_config() {
        let config = b"
            timeout 100
            default 2
            driver_path /efi/drivers
            editor true
            pxe false
            background gray
            foreground white
            highlight_background black
            highlight_foreground white
        ";

        let config = BootConfig::get_boot_config(config, None);
        assert_eq!(config.timeout, 100);
        assert_eq!(config.default, Some(2));
        assert_eq!(config.driver_path, "\\efi\\drivers".to_owned());
        assert!(config.editor);
        assert!(!config.pxe);
        assert!(matches!(config.bg, Color::LightGray));
        assert!(matches!(config.fg, Color::White));
        assert!(matches!(config.highlight_bg, Color::Black));
        assert!(matches!(config.highlight_fg, Color::White));
    }

    proptest! {
        #[test]
        fn doesnt_panic(x in any::<Vec<u8>>(), y in any::<usize>()) {
            let _ = BootConfig::get_boot_config(&x, Some(y));
        }
    }
}

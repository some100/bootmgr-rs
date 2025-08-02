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

use alloc::{borrow::ToOwned, string::String};
use log::warn;
use uefi::{CStr16, boot, cstr16, proto::console::text::Color};

use crate::{
    BootResult,
    system::{
        fs::{check_file_exists, read_into},
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
    pub fn new() -> BootResult<Self> {
        let mut fs = boot::get_image_file_system(boot::image_handle())?;

        if check_file_exists(&mut fs, CONFIG_PATH) {
            let mut buf = [0; 4096]; // a config file over 4096 bytes is very unusual and is not supported
            let bytes = match read_into(&mut fs, CONFIG_PATH, &mut buf) {
                Ok(bytes) => bytes,
                Err(e) => {
                    warn!("{e}");
                    return Ok(Self::default());
                }
            };

            return Ok(Self::get_boot_config(&buf, Some(bytes)));
        }

        Ok(Self::default())
    }

    /// Parses the contents of a [`BootConfig`] format string.
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_boot_config(content: &[u8], bytes: Option<usize>) -> Self {
        let mut config = Self::default();
        let slice = &content[0..bytes.unwrap_or(content.len())];

        if let Ok(content) = str::from_utf8(slice) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once(' ') {
                    let value = value.trim().to_owned();
                    match &*key.to_ascii_lowercase() {
                        "timeout" => {
                            if let Ok(value) = value.parse() {
                                config.timeout = value;
                            }
                        }
                        "default" => {
                            if let Ok(value) = value.parse() {
                                config.default = Some(value);
                            }
                        }
                        "driver_path" => {
                            let value = normalize_path(&value);
                            config.driver_path = value;
                        }
                        "editor" => {
                            if let Ok(value) = value.parse() {
                                config.editor = value;
                            }
                        }
                        "pxe" => {
                            if let Ok(value) = value.parse() {
                                config.pxe = value;
                            }
                        }
                        "background" => config.bg = match_str_color_bg(&value),
                        "foreground" => config.fg = match_str_color_fg(&value),
                        "highlight_background" => config.highlight_bg = match_str_color_bg(&value),
                        "highlight_foreground" => config.highlight_fg = match_str_color_fg(&value),
                        _ => (),
                    }
                }
            }
        }

        config
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self {
            timeout: 5,
            default: None,
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
    use super::*;

    #[test]
    fn test_full_config() {
        let config = r"
            timeout 100
            default 2
            driver_path /efi/drivers
            editor true
            pxe false
            background gray
            foreground white
            highlight_background black
            highlight_foreground white
        "
        .as_bytes();

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
}

//! Provides [`BootConfig`], the configuration file for the bootloader.
//!
//! This parses space separated key value pairs, the format of which is defined in
//! the [`BootConfig`] struct.
//!
//! The general syntax of the configuration file is not too dissimilar from that of BLS configuration files that
//! come with systemd-boot.

use alloc::{borrow::ToOwned, string::String};
use log::warn;
use ratatui_core::style::Color;
use uefi::{CStr16, boot, cstr16};

use crate::{
    BootResult,
    system::{
        fs::{check_file_exists, read_to_string},
        helper::normalize_path,
    },
};

const CONFIG_PATH: &CStr16 = cstr16!("\\loader\\bootmgr-rs.conf");

/// The configuration file for the bootloader.
pub struct BootConfig {
    /// The timeout for the bootloader before the default boot option is selected.
    pub timeout: i64,

    /// The default boot option as the index of the entry.
    pub default: Option<usize>,

    /// The path to the drivers in the same filesystem as the bootloader.
    pub driver_path: String,

    /// Allows for the editor to be enabled.
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
            let content = match read_to_string(&mut fs, CONFIG_PATH) {
                Ok(content) => content,
                Err(e) => {
                    warn!("{e}");
                    return Ok(Self::default());
                }
            };
            return Ok(Self::get_boot_config(&content));
        }

        Ok(Self::default())
    }

    /// Parses the contents of a [`BootConfig`] format string.
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_boot_config(content: &str) -> Self {
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();

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
            highlight_bg: Color::Gray,
            highlight_fg: Color::Black,
        }
    }
}

fn match_str_color_fg(color: &str) -> Color {
    match color {
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" => Color::Gray,
        "dark_gray" => Color::DarkGray,
        "light_red" => Color::LightRed,
        "light_green" => Color::LightGreen,
        "light_yellow" => Color::LightYellow,
        "light_blue" => Color::LightBlue,
        "light_magenta" => Color::LightMagenta,
        "light_cyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Black,
    }
}

fn match_str_color_bg(color: &str) -> Color {
    match color {
        "blue" => Color::Blue,
        "green" => Color::Green,
        "cyan" => Color::Cyan,
        "red" => Color::Red,
        "magenta" => Color::Magenta,
        "gray" | "white" => Color::Gray, // close enough
        _ => Color::Black,
    }
}

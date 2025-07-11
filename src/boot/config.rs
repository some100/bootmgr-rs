use alloc::{borrow::ToOwned, string::String};
use log::warn;
use ratatui_core::style::Color;
use uefi::{CStr16, boot, cstr16};

use crate::system::{
    fs::{check_file_exists, read_to_string},
    helper::normalize_path,
};

const CONFIG_PATH: &CStr16 = cstr16!("\\EFI\\BOOT\\bootmgr-rs.conf");

/// The configuration file for the bootloader.
pub struct BootConfig {
    pub timeout: i64,
    pub default: Option<usize>,
    pub driver_path: String,
    pub editor: bool,
    pub bg: Color,
    pub fg: Color,
    pub highlight_bg: Color,
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
    pub fn new() -> uefi::Result<Self> {
        let mut config = BootConfig::default();
        let mut fs = boot::get_image_file_system(boot::image_handle())?;

        if let Ok(true) = check_file_exists(&mut fs, CONFIG_PATH) {
            let content = match read_to_string(&mut fs, CONFIG_PATH) {
                Ok(content) => content,
                Err(e) => {
                    warn!("{e}");
                    return Ok(config);
                }
            };
            get_boot_config(&mut config, &content);
        }

        Ok(config)
    }
}

// Parses a BootConfig from the content of a file
fn get_boot_config(config: &mut BootConfig, content: &str) {
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
                "background" => config.bg = match_str_color_bg(&value),
                "foreground" => config.fg = match_str_color_fg(&value),
                "highlight_background" => config.highlight_bg = match_str_color_bg(&value),
                "highlight_foreground" => config.highlight_fg = match_str_color_fg(&value),
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
            driver_path: "\\EFI\\BOOT\\drivers".to_owned(),
            editor: false,
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

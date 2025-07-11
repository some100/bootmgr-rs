#![cfg(feature = "windows")]

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;
use nt_hive::{Hive, KeyNode};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    error::BootError,
    system::{
        fs::{check_file_exists, read},
        helper::get_path_cstr,
    },
};

const WIN_PREFIX: &CStr16 = cstr16!("\\EFI\\Microsoft\\Boot");
const WIN_SUFFIX: &str = ".efi";

const DISPLAYORDER_PATH: &str =
    "Objects\\{9dea862c-5cdd-4e70-acc1-f32b344d4795}\\Elements\\24000001";

/// The parser for Windows boot configurations
pub struct WinConfig {
    title: String,
}

impl WinConfig {
    /// # Errors
    ///
    /// May return an `Error` if the provided file is not a [`Hive`], there is not `displayorder`,
    /// and there is no `description` if a `displayorder` does exist, and has a length of 1.
    pub fn new(content: &[u8]) -> Result<Self, BootError> {
        let mut config = WinConfig::default();
        let hive = Hive::new(content)?;
        // may cause a panic due to unchecked subtraction with some malformed inputs
        // this seems to be a bug with nt hive
        let root_key_node = hive.root_key_node()?;
        let displayorder =
            Self::get_values_of_key(DISPLAYORDER_PATH, "displayorder", &root_key_node)?;
        let displayorder_len = displayorder.len();

        if let Some(guid) = displayorder.into_iter().next()
            && displayorder_len == 1
        {
            let path = format!("Objects\\{guid}\\Elements\\12000004");
            let description = Self::get_value_of_key(&path, "description", &root_key_node)?;

            config.title = description;
        }

        Ok(config)
    }

    fn get_value_of_key(
        path: &str,
        key_name: &'static str,
        root_key_node: &KeyNode<'_, &[u8]>,
    ) -> Result<String, BootError> {
        let key = root_key_node
            .subpath(path)
            .ok_or(BootError::BcdMissingKey(key_name))??;
        let value = key
            .value("Element")
            .ok_or(BootError::BcdMissingElement(key_name))??
            .string_data()?;
        Ok(value)
    }

    fn get_values_of_key(
        path: &str,
        key_name: &'static str,
        root_key_node: &KeyNode<'_, &[u8]>,
    ) -> Result<Vec<String>, BootError> {
        let key = root_key_node
            .subpath(path)
            .ok_or(BootError::BcdMissingKey(key_name))??;
        Ok(key
            .value("Element")
            .ok_or(BootError::BcdMissingElement(key_name))??
            .multi_string_data()?
            .filter_map(Result::ok)
            .collect())
    }
}

impl Default for WinConfig {
    fn default() -> Self {
        Self {
            title: "Windows".to_owned(),
        }
    }
}

impl ConfigParser for WinConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        if let Ok(true) = check_file_exists(fs, &get_path_cstr(WIN_PREFIX, cstr16!("BCD"))) {
            match get_win_config(fs, handle) {
                Ok(config) => configs.push(config),
                Err(e) => warn!("{e}"),
            }
        }
    }
}

fn get_win_config(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
) -> Result<Config, BootError> {
    let content = read(fs, &get_path_cstr(WIN_PREFIX, cstr16!("BCD")))?;

    let win_config = WinConfig::new(&content)?;

    let efi = format!("{WIN_PREFIX}\\bootmgfw.efi");
    let config = ConfigBuilder::new(efi, "bootmgfw.efi", WIN_SUFFIX)
        .title(win_config.title)
        .sort_key("windows")
        .handle(handle);

    Ok(config.build())
}
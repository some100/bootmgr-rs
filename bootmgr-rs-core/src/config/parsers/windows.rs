//! A parser for the Windows BCD and Windows boot manager.

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;
use nt_hive::{Hive, KeyNode};
use thiserror::Error;
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    BootResult,
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    system::{
        fs::{check_file_exists, read},
        helper::get_path_cstr,
    },
};

/// The configuration prefix.
const WIN_PREFIX: &CStr16 = cstr16!("\\EFI\\Microsoft\\Boot");

/// The configuration suffix.
const WIN_SUFFIX: &str = ".efi";

/// The path to the `displayorder` element.
const DISPLAYORDER_PATH: &str =
    "Objects\\{9dea862c-5cdd-4e70-acc1-f32b344d4795}\\Elements\\24000001";

/// Errors that may result from parsing the Windows config.
#[derive(Error, Debug)]
pub enum WinError {
    /// The BCD could not be parsed for any reason.
    #[error("Hive Parse Error")]
    Hive(#[from] nt_hive::NtHiveError),

    /// The BCD was missing a required key for parsing.
    #[error("BCD missing key: \"{0}\"")]
    BcdMissingKey(&'static str),

    /// The BCD was missing a required value inside of a key for parsing.
    #[error("BCD missing Element value in key: \"{0}\"")]
    BcdMissingElement(&'static str),
}

/// The parser for Windows boot configurations
pub struct WinConfig {
    /// The title of the Windows configuration, if found.
    title: String,
}

impl WinConfig {
    /// Creates a new [`WinConfig`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the provided file is not a [`Hive`], there is not `displayorder`,
    /// and there is no `description` if a `displayorder` does exist, and has a length of 1.
    pub fn new(content: &[u8]) -> Result<Self, WinError> {
        let mut config = Self::default();
        let hive = Hive::new(content)?;
        // may cause a panic due to unchecked subtraction with some malformed inputs
        // this seems to be a bug with nt hive, nothing can really be done from here without using
        // a new crate or a custom implementation
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

    /// Get the [`String`] value of a certain key.
    ///
    /// This parses the `Element` value of a key as a singular [`String`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the BCD is missing that key, the BCD is missing the `Element` value,
    /// or the value is not `REG_SZ` or `REG_EXPAND_SZ`.
    fn get_value_of_key(
        path: &str,
        key_name: &'static str,
        root_key_node: &KeyNode<'_, &[u8]>,
    ) -> Result<String, WinError> {
        let key = root_key_node
            .subpath(path)
            .ok_or(WinError::BcdMissingKey(key_name))??;
        let value = key
            .value("Element")
            .ok_or(WinError::BcdMissingElement(key_name))??
            .string_data()?;
        Ok(value)
    }

    /// Get the [`String`] values of a certain key.
    ///
    /// This parses the `Element` value of a key as a vector of [`String`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the BCD is missing that key, the BCD is missing the `Element` value,
    /// or the value is not `REG_MULTI_SZ`.
    fn get_values_of_key(
        path: &str,
        key_name: &'static str,
        root_key_node: &KeyNode<'_, &[u8]>,
    ) -> Result<Vec<String>, WinError> {
        let key = root_key_node
            .subpath(path)
            .ok_or(WinError::BcdMissingKey(key_name))??;
        Ok(key
            .value("Element")
            .ok_or(WinError::BcdMissingElement(key_name))??
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
        let Ok(path) = get_path_cstr(WIN_PREFIX, cstr16!("BCD")) else {
            return;
        };
        if check_file_exists(fs, &path) {
            match get_win_config(fs, handle) {
                Ok(config) => configs.push(config),
                Err(e) => warn!("{e}"),
            }
        }
    }
}

/// Parse a BLS file given a [`SimpleFileSystem`] protocol, and a handle to that protocol.
fn get_win_config(fs: &mut ScopedProtocol<SimpleFileSystem>, handle: Handle) -> BootResult<Config> {
    let content = read(fs, &get_path_cstr(WIN_PREFIX, cstr16!("BCD"))?)?;

    let win_config = WinConfig::new(&content)?;

    let efi = format!("{WIN_PREFIX}\\bootmgfw.efi");
    let config = ConfigBuilder::new("bootmgfw.efi", WIN_SUFFIX)
        .efi(efi)
        .title(win_config.title)
        .sort_key("windows")
        .handle(handle)
        .origin(Parsers::Windows);

    Ok(config.build())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn doesnt_panic(x in any::<Vec<u8>>()) {
            let _ = WinConfig::new(&x);
        }
    }
}

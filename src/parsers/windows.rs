use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;
use nt_hive::Hive;
use uefi::{CStr16, Handle, cstr16, fs::FileSystem};

use crate::{
    error::BootError,
    parsers::{Config, ConfigParser},
    system::helper::get_path_cstr,
};

const WIN_PREFIX: &CStr16 = cstr16!("\\EFI\\Microsoft\\Boot");

pub struct WinConfig {
    title: String,
}

impl WinConfig {
    fn new(content: Vec<u8>) -> Result<Self, BootError> {
        let mut config = WinConfig::default();
        let hive = Hive::new(content.as_ref())?;
        let root_key_node = hive.root_key_node()?;
        let displayorder_key = root_key_node
            .subpath("Objects\\{9dea862c-5cdd-4e70-acc1-f32b344d4795}\\Elements\\24000001")
            .ok_or(BootError::Generic("BCD does not contain displayorder key"))??;
        let displayorder: Vec<_> = displayorder_key
            .value("Element")
            .ok_or(BootError::Generic(
                "No subkey named Element inside displayorder key",
            ))??
            .multi_string_data()?
            .collect();

        if displayorder.len() == 1 {
            let guid = displayorder.into_iter().next().unwrap()?; // unwrapping should be fine, since len is 1
            let path = format!("Objects\\{guid}\\Elements\\12000004");
            let description_key = root_key_node
                .subpath(&path)
                .ok_or(BootError::Generic("BCD does not contain description key"))??;
            let description = description_key
                .value("Element")
                .ok_or(BootError::Generic(
                    "No subkey named Element inside description key",
                ))??
                .string_data()?;

            config.title = description;
        }

        Ok(config)
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
    fn parse_configs(fs: &mut FileSystem, handle: &Handle, configs: &mut Vec<Config>) {
        let content = match fs.read(&*get_path_cstr(WIN_PREFIX, cstr16!("BCD"))) {
            Ok(content) => content,
            Err(e) => {
                warn!("{e}");
                return;
            }
        };

        let win_config = match WinConfig::new(content) {
            Ok(win_config) => win_config,
            Err(e) => {
                warn!("{e}");
                return;
            }
        };

        let config = Config {
            title: Some(win_config.title),
            sort_key: Some("windows".to_owned()),
            efi: Some(format!("{WIN_PREFIX}\\bootmgfw.efi")),
            handle: Some(*handle),
            filename: "bootmgfw.efi".to_owned(),
            suffix: ".efi".to_owned(),
            ..Config::default()
        };

        configs.push(config);
    }
}

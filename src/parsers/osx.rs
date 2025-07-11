use alloc::{borrow::ToOwned, format, vec::Vec};
use uefi::{CStr16, Handle, cstr16, fs::FileSystem};

use crate::{
    parsers::{Config, ConfigParser},
    system::helper::get_path_cstr,
};

const BOOTEFI_PREFIX: &CStr16 = cstr16!("\\System\\Library\\CoreServices");

pub struct OsxConfig;

impl ConfigParser for OsxConfig {
    fn parse_configs(fs: &mut FileSystem, handle: &Handle, configs: &mut Vec<Config>) {
        if let Ok(true) = fs.try_exists(&*get_path_cstr(BOOTEFI_PREFIX, cstr16!("boot.efi"))) {
            let config = Config {
                title: Some("macOS".to_owned()),
                sort_key: Some("macos".to_owned()),
                efi: Some(format!("{BOOTEFI_PREFIX}\\boot.efi")),
                handle: Some(*handle),
                filename: "boot.efi".to_owned(),
                suffix: ".efi".to_owned(),
                ..Config::default()
            };

            configs.push(config);
        }
    }
}

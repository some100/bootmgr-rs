//! An auto detector for the macOS boot loader.
#![cfg(feature = "osx")]

use alloc::{format, vec::Vec};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{fs::check_file_exists, helper::get_path_cstr},
};

/// The configuration prefix.
const BOOTEFI_PREFIX: &CStr16 = cstr16!("\\System\\Library\\CoreServices");

/// The configuration suffix.
const BOOTEFI_SUFFIX: &str = ".efi";

/// A "parser" for detecting macOS boot configurations
pub struct OsxConfig;

impl ConfigParser for OsxConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let Ok(path) = get_path_cstr(BOOTEFI_PREFIX, cstr16!("boot.efi")) else {
            return; // this should not happen, the path is hardcoded and valid
        };

        if check_file_exists(fs, &path) {
            let efi = format!("{BOOTEFI_PREFIX}\\boot.efi");
            let config = ConfigBuilder::new("boot.efi", BOOTEFI_SUFFIX)
                .efi(efi)
                .title("macOS")
                .sort_key("macos")
                .handle(handle);

            configs.push(config.build());
        }
    }
}

//! An auto detector for the UEFI shell (located at /shellx64.efi)
#![cfg(feature = "shell")]

use alloc::{format, vec::Vec};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{fs::check_file_exists, helper::get_path_cstr},
};

/// The configuration prefix.
const SHELL_PREFIX: &CStr16 = cstr16!(""); // the root of the partition

/// The configuration suffix.
const SHELL_SUFFIX: &str = ".efi";

/// A "parser" for detecting shellx64.efi
pub struct ShellConfig;

impl ConfigParser for ShellConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let Ok(path) = get_path_cstr(SHELL_PREFIX, cstr16!("shellx64.efi")) else {
            return; // this should not happen as the path is hardcoded and valid
        };
        if check_file_exists(fs, &path) {
            let efi = format!("{SHELL_PREFIX}\\shellx64.efi");
            let config = ConfigBuilder::new("shellx64.efi", SHELL_SUFFIX)
                .efi(efi)
                .title("UEFI Shell")
                .sort_key("shell")
                .handle(handle);

            configs.push(config.build());
        }
    }
}

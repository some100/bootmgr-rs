#![cfg(feature = "shell")]

use alloc::{format, vec::Vec};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{fs::check_file_exists, helper::get_path_cstr},
};

const SHELL_PREFIX: &CStr16 = cstr16!(""); // the root of the partition
const SHELL_SUFFIX: &str = ".efi";

/// A "parser" for detecting shellx64.efi
pub struct ShellConfig;

impl ConfigParser for ShellConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        if let Ok(true) =
            check_file_exists(fs, &get_path_cstr(SHELL_PREFIX, cstr16!("shellx64.efi")))
        {
            let efi = format!("{SHELL_PREFIX}\\shellx64.efi");
            let config = ConfigBuilder::new(efi, "shellx64.efi", SHELL_SUFFIX)
                .title("UEFI Shell")
                .sort_key("shell")
                .handle(handle);

            configs.push(config.build());
        }
    }
}

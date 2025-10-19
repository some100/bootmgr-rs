// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! An auto detector for the macOS boot loader.

use alloc::vec::Vec;

use const_format::formatcp;
use uefi::{CStr16, Handle, cstr16};

use crate::{
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    system::{fs::UefiFileSystem, helper::get_path_cstr},
};

/// The configuration prefix.
const BOOTEFI_PREFIX: &CStr16 = cstr16!("\\System\\Library\\CoreServices");

/// The configuration prefix as an &str.
const BOOTEFI_PREFIX_STR: &str = "\\System\\Library\\CoreServices";

/// The configuration suffix.
const BOOTEFI_SUFFIX: &str = ".efi";

/// A "parser" for detecting macOS boot configurations
pub struct OsxConfig;

impl ConfigParser for OsxConfig {
    fn parse_configs(fs: &mut UefiFileSystem, handle: Handle, configs: &mut Vec<Config>) {
        let Ok(path) = get_path_cstr(BOOTEFI_PREFIX, cstr16!("boot.efi")) else {
            return;
        };

        if fs.exists(&path) {
            let efi_path = formatcp!("{BOOTEFI_PREFIX_STR}\\boot.efi");
            let config = ConfigBuilder::new("boot.efi", BOOTEFI_SUFFIX)
                .efi_path(efi_path)
                .title("macOS")
                .sort_key("macos")
                .fs_handle(handle)
                .origin(Parsers::Osx);

            configs.push(config.build());
        }
    }
}

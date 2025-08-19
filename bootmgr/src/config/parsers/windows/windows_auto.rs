// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! An auto detector for the Microsoft Boot Manager (bootmgfw.efi)

use alloc::{format, vec::Vec};
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
const WIN_PREFIX: &CStr16 = cstr16!("\\EFI\\Microsoft\\Boot");

/// The configuration suffix.
const WIN_SUFFIX: &str = ".efi";

/// A "parser" for detecting bootmgfw.efi
pub struct WinConfig;

impl ConfigParser for WinConfig {
    fn parse_configs(fs: &mut UefiFileSystem, handle: Handle, configs: &mut Vec<Config>) {
        let Ok(path) = get_path_cstr(WIN_PREFIX, cstr16!("bootmgfw.efi")) else {
            return;
        };
        if fs.exists(&path) {
            let efi_path = format!("{WIN_PREFIX}\\bootmgfw.efi");
            let config = ConfigBuilder::new("bootmgfw.efi", WIN_SUFFIX)
                .efi_path(efi_path)
                .title("Windows Boot Manager")
                .sort_key("windows")
                .fs_handle(handle)
                .origin(Parsers::Windows);

            configs.push(config.build());
        }
    }
}

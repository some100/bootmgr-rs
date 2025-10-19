// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! An auto detector for the fallback boot loader (BOOTx64.efi, etc.)

use alloc::vec::Vec;

use const_format::formatcp;
use uefi::{CStr16, Handle, cstr16};

use crate::{
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    system::{
        fs::UefiFileSystem,
        helper::{get_path_cstr, str_to_cstr},
    },
};

/// The configuration prefix.
const FALLBACK_PREFIX: &CStr16 = cstr16!("\\EFI\\BOOT");

/// The configuration prefix as an &str.
const FALLBACK_PREFIX_STR: &str = "\\EFI\\BOOT";

/// The configuration suffix.
const FALLBACK_SUFFIX: &str = ".efi";

/// The filename of the fallback boot program for the architecture.
const FILENAME: &str = get_filename();

/// A "parser" for detecting BOOTx64.efi, BOOTia32.efi, BOOTaa32.efi, BOOTaa64.efi
pub struct FallbackConfig;

impl ConfigParser for FallbackConfig {
    fn parse_configs(fs: &mut UefiFileSystem, handle: Handle, configs: &mut Vec<Config>) {
        let Ok(filename) = str_to_cstr(FILENAME) else {
            return; // there is no way this can fail, as filename can only be one of four strings
        };

        let Ok(path) = get_path_cstr(FALLBACK_PREFIX, &filename) else {
            return; // this also should not fail, since this path is hardcoded and valid
        };

        if fs.exists(&path)
            && let Ok(volume_label) = fs.get_volume_label()
        {
            let efi_path = formatcp!("{FALLBACK_PREFIX_STR}\\{FILENAME}");
            let title = if volume_label.is_empty() {
                &filename
            } else {
                &volume_label // prefer the volume label if it exists, so we can tell the difference between fallbacks
            };
            let config = ConfigBuilder::new(&filename, FALLBACK_SUFFIX)
                .efi_path(efi_path)
                .title(title)
                .sort_key("fallback")
                .fs_handle(handle)
                .origin(Parsers::Fallback);

            configs.push(config.build());
        }
    }
}

/// Get the filename of the fallback boot program for the current architecture.
const fn get_filename() -> &'static str {
    if cfg!(target_arch = "x86") {
        "BOOTia32.efi"
    } else if cfg!(target_arch = "x86_64") {
        "BOOTx64.efi"
    } else if cfg!(target_arch = "arm") {
        "BOOTaa32.efi"
    } else if cfg!(target_arch = "aarch64") {
        "BOOTaa64.efi"
    } else {
        ""
    }
}

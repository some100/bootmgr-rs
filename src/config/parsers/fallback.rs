#![cfg(feature = "fallback")]

use alloc::{format, vec::Vec};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{
        fs::{check_file_exists, get_volume_label},
        helper::{get_arch, get_path_cstr, str_to_cstr},
    },
};

const FALLBACK_PREFIX: &CStr16 = cstr16!("\\EFI\\BOOT");
const FALLBACK_SUFFIX: &str = ".efi";

/// A "parser" for detecting BOOTx64.efi, BOOTia32.efi, BOOTaa32.efi, BOOTaa64.efi
pub struct FallbackConfig;

impl ConfigParser for FallbackConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let filename = match get_arch().as_deref() {
            Some("x86") => "BOOTia32.efi",
            Some("x64") => "BOOTx64.efi",
            Some("arm") => "BOOTaa32.efi",
            Some("aa64") => "BOOTaa64.efi",
            _ => return,
        };
        let filename = str_to_cstr(filename);

        if let Ok(true) = check_file_exists(fs, &get_path_cstr(FALLBACK_PREFIX, &filename))
            && let Ok(volume_label) = get_volume_label(fs)
        {
            let efi = format!("{FALLBACK_PREFIX}\\{filename}");
            let title = if volume_label.is_empty() {
                &filename
            } else {
                &volume_label // prefer the volume label if it exists, so we can tell the difference between fallbacks
            };
            let config = ConfigBuilder::new(efi, &filename, FALLBACK_SUFFIX)
                .title(title)
                .sort_key("fallback")
                .handle(handle);

            configs.push(config.build());
        }
    }
}

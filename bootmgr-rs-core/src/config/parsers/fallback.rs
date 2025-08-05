//! An auto detector for the fallback boot loader (BOOTx64.efi, etc.)

use alloc::{format, vec::Vec};
use uefi::{CStr16, Handle, boot::ScopedProtocol, cstr16, proto::media::fs::SimpleFileSystem};

use crate::{
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    system::{
        fs::{check_file_exists, get_volume_label},
        helper::{get_arch, get_path_cstr, str_to_cstr},
    },
};

/// The configuration prefix.
const FALLBACK_PREFIX: &CStr16 = cstr16!("\\EFI\\BOOT");

/// The configuration suffix.
const FALLBACK_SUFFIX: &str = ".efi";

/// A "parser" for detecting BOOTx64.efi, BOOTia32.efi, BOOTaa32.efi, BOOTaa64.efi
pub struct FallbackConfig;

impl ConfigParser for FallbackConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let filename = match get_arch().as_deref().map(alloc::string::String::as_str) {
            Some("x86") => "BOOTia32.efi",
            Some("x64") => "BOOTx64.efi",
            Some("arm") => "BOOTaa32.efi",
            Some("aa64") => "BOOTaa64.efi",
            _ => return,
        };

        let Ok(filename) = str_to_cstr(filename) else {
            return; // there is no way this can fail, as filename can only be one of four strings
        };

        let Ok(path) = get_path_cstr(FALLBACK_PREFIX, &filename) else {
            return; // this also should not fail, since this path is hardcoded and valid
        };

        if check_file_exists(fs, &path)
            && let Ok(volume_label) = get_volume_label(fs)
        {
            let efi_path = format!("{FALLBACK_PREFIX}\\{filename}");
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

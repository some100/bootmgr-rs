use alloc::vec::Vec;
use uefi::{Handle, boot::ScopedProtocol, proto::media::fs::SimpleFileSystem};

use crate::config::Config;

pub mod bls;
pub mod fallback;
pub mod osx;
pub mod shell;
pub mod uki;
pub mod windows;

/// Parses configs.
pub trait ConfigParser {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    );
}

/// Parses every config file that has an implementation in parsers.
pub fn parse_all_configs(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
    configs: &mut Vec<Config>,
) {
    #[cfg(feature = "bls")]
    bls::BlsConfig::parse_configs(fs, handle, configs);

    #[cfg(feature = "fallback")]
    fallback::FallbackConfig::parse_configs(fs, handle, configs);

    #[cfg(feature = "osx")]
    osx::OsxConfig::parse_configs(fs, handle, configs);

    #[cfg(feature = "shell")]
    shell::ShellConfig::parse_configs(fs, handle, configs);

    #[cfg(feature = "uki")]
    uki::UkiConfig::parse_configs(fs, handle, configs);

    #[cfg(feature = "windows")]
    windows::WinConfig::parse_configs(fs, handle, configs);
}

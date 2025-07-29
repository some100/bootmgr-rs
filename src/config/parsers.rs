//! Parses configuration files of various formats into [`Config`].
//!
//! The currently supported formats are as follows:
//! - BLS Config files (also known as BLS Type 1)
//! - UKI Executable files (also known as BLS Type 2)
//! - Windows BCD
//!
//! This also supports auto detection for:
//! - BOOTx64.efi, BOOTia32.efi, BOOTaa32.efi, BOOTaa64.efi.
//! - shellx64.efi
//! - boot.efi (macOS)

use alloc::vec::Vec;
use uefi::{Handle, boot::ScopedProtocol, proto::media::fs::SimpleFileSystem};

use crate::config::Config;

/// The BLS (BLS type 1) parser.
pub mod bls;

/// The fallback boot EFI detector.
pub mod fallback;

/// The macOS boot EFI detector.
pub mod osx;

/// The UEFI shell boot EFI detector.
pub mod shell;

/// The UKI (BLS type 2) EFI parser.
pub mod uki;

/// The Windows BCD parser.
pub mod windows;

/// Parses configs.
pub trait ConfigParser {
    /// Pushes configs into a mutable reference to a vector, given a filesystem and handle to that filesystem.
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

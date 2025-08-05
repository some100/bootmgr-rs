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

use crate::{config::Config, features};

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

/// The parsers that exist.
#[derive(Clone, Copy, Debug)]
pub enum Parsers {
    /// The BLS Type #1 parser.
    Bls,

    /// The fallback bootloader autodetection.
    Fallback,

    /// The `boot.efi` macOS autodetection.
    Osx,

    /// The UEFI shell autodetection.
    Shell,

    /// The BLS Type #2 (UKI) parser.
    Uki,

    /// The Windows BCD parser.
    Windows,

    /// A special boot option (such as reboot, shutdown).
    Special,
}

impl Parsers {
    /// Convert a [`Parsers`] type into an [`&str`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn as_str(self) -> &'static str {
        match self {
            Parsers::Bls => "bls",
            Parsers::Fallback => "fallback",
            Parsers::Osx => "osx",
            Parsers::Shell => "shell",
            Parsers::Uki => "uki",
            Parsers::Windows => "windows",
            Parsers::Special => "special",
        }
    }
}

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
pub(super) fn parse_all_configs(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
    configs: &mut Vec<Config>,
) {
    features::bls::BlsConfig::parse_configs(fs, handle, configs);
    features::fallback::FallbackConfig::parse_configs(fs, handle, configs);
    features::osx::OsxConfig::parse_configs(fs, handle, configs);
    features::shell::ShellConfig::parse_configs(fs, handle, configs);
    features::uki::UkiConfig::parse_configs(fs, handle, configs);
    features::windows::WinConfig::parse_configs(fs, handle, configs);
}

//! Provides [`BootAction`], which allows special actions to be done when an entry is loaded

use alloc::{borrow::ToOwned, vec::Vec};
use uefi::Handle;

use crate::{
    BootResult,
    boot::{config::BootConfig, loader},
    config::Config,
};

pub mod firmware;
pub mod pxe;
pub mod reboot;
pub mod shutdown;

/// Actions that decide which boot loader to use.
///
/// This also handles the special cases of rebooting, shutting down, and resetting to firmware.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BootAction {
    /// Boot using the EFI boot loader.
    #[default]
    BootEfi,

    /// Boot using the TFTP boot loader.
    BootTftp,

    /// Reboot the system.
    Reboot,

    /// Shut down the system.
    Shutdown,

    /// Reboot the system into firmware setup.
    ResetToFirmware,
}

impl BootAction {
    /// Runs a boot action given a config.
    ///
    /// # Errors
    ///
    /// May return an `Error` if any of the actions fail.
    pub fn run(&self, config: &Config) -> BootResult<Handle> {
        match self {
            BootAction::Reboot => reboot::reset(),
            BootAction::Shutdown => shutdown::shutdown(),
            BootAction::ResetToFirmware => firmware::reset_to_firmware(),
            BootAction::BootEfi => loader::efi::load_boot_option(config),
            BootAction::BootTftp => loader::tftp::load_boot_option(config),
        }
    }
}

/// Adds reboot, shutdown, reset into firmware, and optionally a PXE boot entry.
pub fn add_special_boot(configs: &mut Vec<Config>, boot_config: &BootConfig) {
    let actions = [
        ("Reboot", BootAction::Reboot),
        ("Shutdown", BootAction::Shutdown),
        (
            "Reboot Into Firmware Interface",
            BootAction::ResetToFirmware,
        ),
    ];

    for (title, action) in actions {
        let config = Config {
            title: Some(title.to_owned()),
            action,
            ..Config::default()
        };

        configs.push(config);
    }

    if boot_config.pxe
        && let Ok(Some(config)) = pxe::get_pxe_offer()
    {
        configs.push(config);
    }
}

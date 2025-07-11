//! Provides [`BootAction`], which allows special actions to be done when an entry is loaded

use alloc::{borrow::ToOwned, vec::Vec};
use uefi::{
    Status,
    runtime::{self, ResetType},
};

use crate::{boot::action::firmware::reset_to_firmware, config::Config};

pub mod firmware;

#[derive(Clone, Debug, Default)]
pub enum BootAction {
    #[default]
    Boot,
    Reboot,
    Shutdown,
    ResetToFirmware,
}

/// Performs an action (reboot, shutdown, reset to firmware) depending on the passed boot action.
///
/// # Errors
///
/// May return an `Error` if resetting to firmware fails.
pub fn handle_action(action: &BootAction) -> uefi::Result<()> {
    match action {
        BootAction::Reboot => runtime::reset(ResetType::WARM, Status::SUCCESS, None),
        BootAction::Shutdown => runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None),
        BootAction::ResetToFirmware => reset_to_firmware(),
        BootAction::Boot => Ok(()),
    }
}

/// Adds reboot, shutdown, and reset into firmware setup boot entries.
pub fn add_special_boot(configs: &mut Vec<Config>) {
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
}

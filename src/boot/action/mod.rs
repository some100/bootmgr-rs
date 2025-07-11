use alloc::{borrow::ToOwned, vec::Vec};

use crate::parsers::Config;

pub mod firmware;

#[derive(Clone, Debug)]
pub enum BootAction {
    Boot,
    Reboot,
    Shutdown,
    ResetToFirmware,
}

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

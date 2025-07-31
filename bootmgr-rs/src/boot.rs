//! Provides [`BootMgr`], a struct which abstracts most of loading a [`Config`].

use alloc::vec::Vec;
use log::error;
use uefi::{Handle, cstr16};

use crate::{
    BootResult,
    boot::{action::add_special_boot, config::BootConfig, loader::load_boot_option},
    config::{Config, get_configs},
    system::{
        drivers::load_drivers,
        variable::{get_variable, set_variable},
    },
};

pub mod action;
pub mod config;
pub mod devicetree;
pub mod loader;
pub mod secure_boot;

/// The storage for configuration files.
pub struct BootMgr {
    /// The configuration of the boot manager.
    pub boot_config: BootConfig,

    /// The boot options.
    pub configs: Vec<Config>,
}

impl BootMgr {
    /// Creates a new [`BootMgr`], load drivers, then populate it with [`Config`]s.
    ///
    /// It will also add special boot options, like Reboot, Shutdown, and Reset to Firmware.
    /// This will also parse the main configuration file located at `\\loader\\bootmgr-rs.conf`
    /// for user settings.
    ///
    /// # Errors
    ///
    /// May return an `Error` if a fatal error occurred when parsing the [`BootConfig`] (such as the image handle not
    /// supporting `SimpleFileSystem`) or when parsing the [`Config`]s.
    pub fn new() -> BootResult<Self> {
        let boot_config = BootConfig::new()?;
        load_drivers(&boot_config.driver_path)?; // load drivers before configs from other fs are parsed
        let mut configs = get_configs()?;
        add_special_boot(&mut configs, &boot_config);

        Ok(Self {
            boot_config,
            configs,
        })
    }

    /// Load a boot option from a [`Config`] given the index.
    ///
    /// # Errors
    ///
    /// May return an `Error` if an error occurred while loading the boot option.
    pub fn load(&mut self, selected: usize) -> BootResult<Handle> {
        let config = &self.configs[selected];
        match load_boot_option(config) {
            Ok(handle) => Ok(handle),
            Err(e) => {
                self.configs[selected].bad = true;
                Err(e)
            }
        }
    }

    /// Returns a clone of the inner [`Vec<Config>`].
    #[must_use = "Has no effect if the result is unused"]
    pub fn list(&self) -> Vec<Config> {
        self.configs.clone()
    }

    /// Returns a mutable reference to an inner [`Config`].
    pub fn get_config(&mut self, option: usize) -> &mut Config {
        &mut self.configs[option]
    }

    /// Gets the default boot option.
    ///
    /// It does this in the following order:
    /// 1. UEFI variable
    /// 2. Config file
    ///
    /// If the default boot option is set in neither, then 0 is returned
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_default(&self) -> usize {
        if let Ok(idx) = get_variable::<usize>(cstr16!("BootDefault"), None)
            && idx < self.configs.len()
        {
            return idx;
        }

        if let Some(idx) = self.boot_config.default
            && idx < self.configs.len()
        {
            return idx;
        }

        0
    }

    /// Sets the default boot option by index.
    ///
    /// This is stored in a UEFI variable, so it may not be completely reliable across firmware implementations.
    pub fn set_default(&self, option: usize) {
        if option < self.configs.len()
            && let Err(e) = set_variable::<usize>(cstr16!("BootDefault"), None, None, Some(option))
        {
            error!("Failed to set BootDefault UEFI variable: {e}");
        }
    }

    /// Validates the inner [`Vec<Config>`] through various criteria.
    ///
    /// If any of the [`Config`]s are found to be invalid, then they will be
    /// filtered.
    pub fn validate(&mut self) {
        self.configs.retain_mut(Config::is_good);
    }
}

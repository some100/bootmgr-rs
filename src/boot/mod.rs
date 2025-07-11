use alloc::vec::Vec;
use uefi::{Handle, cstr16};

use crate::{
    boot::{action::add_special_boot, loader::efi},
    error::BootError,
    parsers::{Config, get_configs},
    system::helper::{get_variable_num, set_variable_num},
};

mod devicetree;
mod loader;

pub mod action;

pub struct BootMgr {
    pub configs: Vec<Config>,
}

impl BootMgr {
    pub fn new() -> Result<Self, BootError> {
        let mut configs = get_configs()?;
        add_special_boot(&mut configs);

        Ok(Self { configs })
    }

    pub fn load(&self, selected: usize) -> Result<Handle, BootError> {
        let config = &self.configs[selected];
        efi::load_boot_option(&config)
    }

    pub fn list(&self) -> Vec<Config> {
        self.configs.clone()
    }

    pub fn get_default(&self) -> usize {
        match get_variable_num(cstr16!("BootDefault")) {
            Ok(option) => option.min(self.configs.len()),
            Err(_) => 0, // either wasnt found or something bad with firmware happened. either way nothing we can realistically do, so just return 0
        }
    }

    pub fn set_default(&self, option: usize) {
        if option < self.configs.len() {
            let _ = set_variable_num(cstr16!("BootDefault"), option); // nothing we can do if it fails, so ignore
        }
    }
}

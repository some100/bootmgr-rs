use alloc::{string::String, vec::Vec};
use log::{error, warn};
use uefi::{
    Handle,
    boot::{self, SearchType},
    fs::FileSystem,
    proto::media::fs::SimpleFileSystem,
};

use crate::{
    boot::action::BootAction,
    parsers::{bls::BlsConfig, osx::OsxConfig, uki::UkiConfig, windows::WinConfig},
    system::helper::{get_arch, is_target_partition},
};

mod bls;
mod osx;
mod uki;
mod windows;

#[derive(Clone, Debug)]
pub struct Config {
    pub title: Option<String>,
    pub version: Option<String>,
    pub machine_id: Option<String>,
    pub sort_key: Option<String>,
    pub linux: Option<String>,
    pub efi: Option<String>,
    pub options: Option<String>,
    pub devicetree: Option<String>,
    pub architecture: Option<String>,

    pub bad: bool,
    pub action: BootAction,
    pub handle: Option<Handle>,
    pub filename: String,
    pub suffix: String,
}

impl Config {
    fn validate(&self) -> bool {
        if self.title.is_none() {
            warn!("Config {} does not have a title", self.filename);
        }
        if self.linux.is_none() && self.efi.is_none() {
            error!("Config {} does not contain linux or efi key", self.filename);
            return false;
        }
        if !self.filename.is_ascii() {
            error!(
                "Config {} filename contains invalid characters",
                self.filename
            );
            return false;
        }
        if self.architecture != get_arch() && self.architecture.is_some() {
            warn!(
                "Config {} has non-matching architecture, ignoring",
                self.filename
            );
            return false; // this is a warn, but filter it anyways
        }
        true
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            title: None,
            version: None,
            machine_id: None,
            sort_key: None,
            linux: None,
            efi: None,
            options: None,
            devicetree: None,
            architecture: None,
            bad: false,
            action: BootAction::Boot,
            handle: None,
            filename: String::new(),
            suffix: String::new(),
        }
    }
}

pub trait ConfigParser {
    fn parse_configs(fs: &mut FileSystem, handle: &Handle, configs: &mut Vec<Config>);
}

pub fn get_configs() -> uefi::Result<Vec<Config>> {
    let mut configs = Vec::new();
    let handles =
        boot::locate_handle_buffer(SearchType::from_proto::<SimpleFileSystem>())?.to_vec();

    for handle in handles.into_iter() {
        if !is_target_partition(&handle) {
            continue;
        }

        let fs_proto = boot::open_protocol_exclusive(handle)?;
        let mut fs = FileSystem::new(fs_proto);
        BlsConfig::parse_configs(&mut fs, &handle, &mut configs);
        OsxConfig::parse_configs(&mut fs, &handle, &mut configs);
        UkiConfig::parse_configs(&mut fs, &handle, &mut configs);
        WinConfig::parse_configs(&mut fs, &handle, &mut configs);
    }

    configs = configs
        .into_iter()
        .filter(|config| config.validate())
        .collect();

    configs.sort_by(|a, b| {
        a.bad
            .cmp(&b.bad) // derank bad entries
            .then_with(|| b.sort_key.is_some().cmp(&a.sort_key.is_some())) // always sort entries with sort keys earlier
            .then_with(|| a.sort_key.cmp(&b.sort_key)) // sort by sort key first
            .then_with(|| a.machine_id.cmp(&b.machine_id)) // if equal, sort by machine id second
            .then_with(|| b.version.cmp(&a.version)) // if equal, sort by version third
            .then_with(|| {
                b.filename
                    .strip_suffix(&b.suffix)
                    .cmp(&a.filename.strip_suffix(&a.suffix))
            }) // sort by filename last with suffix removed
    });

    Ok(configs)
}

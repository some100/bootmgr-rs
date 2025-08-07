//! Provides [`Config`], the main configuration struct.
//!
//! This will generally represent a boot entry in the boot manager.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use log::{error, warn};
use smallvec::{SmallVec, smallvec};
use thiserror::Error;
use uefi::{
    boot::{self, SearchType},
    proto::media::fs::SimpleFileSystem,
};

use crate::{
    BootResult,
    boot::action::BootAction,
    config::{
        parsers::{Parsers, parse_all_configs},
        types::{Architecture, DevicetreePath, EfiPath, FsHandle, MachineId, SortKey},
    },
    system::{
        fs::{check_file_exists_str, is_target_partition},
        helper::get_arch,
    },
};

pub mod builder;
pub mod parsers;
pub mod types;

/// Errors indicating that a [`Config`] is invalid.
#[derive(Error, Debug)]
pub enum ConfigError {
    /// There was no `Handle` when one was required.
    #[error("Config \"{0}\" missing handle")]
    ConfigMissingHandle(String),

    /// There was no EFI executable specified when one was required.
    #[error("Config \"{0}\" missing EFI")]
    ConfigMissingEfi(String),

    /// The [`Config`]'s architecture field did not match the system architecture.
    #[error("Config \"{0}\" has non-matching architecture")]
    NonMatchingArch(String),

    /// The path specified by the [`Config`] does not exist.
    #[error("\"{0}\" does not exist at path \"{1}\"")]
    NotExist(&'static str, String),
}

/// The standard [`Config`]
#[derive(Clone, Debug, Default)]
pub struct Config {
    /// The preferred boot name of the entry.
    pub title: Option<String>,

    /// The version of the entry for sorting.
    pub version: Option<String>,

    /// The machine-id for sorting.
    pub machine_id: Option<MachineId>,

    /// The sort-key for sorting.
    pub sort_key: Option<SortKey>,

    /// The options specified in loading the image.
    pub options: Option<String>,

    /// The path to a devicetree, if one is required.
    pub devicetree_path: Option<DevicetreePath>,

    /// The architecture of the entry for filtering.
    pub architecture: Option<Architecture>,

    /// The path to an EFI executable, if one is required.
    pub efi_path: Option<EfiPath>,

    /// The [`BootAction`] of the entry, for deciding which loader to use.
    pub action: BootAction,

    /// Checks if an entry is bad, for sorting and deranking.
    pub bad: bool,

    /// The [`FsHandle`] of the entry, if one is required.
    pub fs_handle: Option<FsHandle>,

    /// The parser from which the entry originated from, if there was one.
    pub origin: Option<Parsers>,

    /// The filename of the entry.
    pub filename: String,

    /// The suffix of the filename of the entry.
    pub suffix: String,
}

impl Config {
    /// Returns an iterator over every [`String`] struct field that should be edited
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_str_fields(&self) -> impl Iterator<Item = (&'static str, Option<&String>)> {
        let vec: SmallVec<[_; 8]> = smallvec![
            ("title", self.title.as_ref()),
            ("version", self.version.as_ref()),
            ("machine_id", self.machine_id.as_deref()),
            ("sort_key", self.sort_key.as_deref()),
            ("options", self.options.as_ref()),
            ("devicetree", self.devicetree_path.as_deref()),
            ("architecture", self.architecture.as_deref()),
            ("efi", self.efi_path.as_deref()),
        ];
        vec.into_iter()
    }

    /// Verifies if a [`Config`] is good. If the [`Config`] is good, then
    /// it will return true. Otherwise, it will return `false`.
    pub fn is_good(&mut self) -> bool {
        self.lint();
        self.validate().map_err(|e| error!("{e}")).is_ok()
    }

    /// Validates a [`Config`], returning an `Error` if any of the "fail" criteria
    /// are met. This ensures that any of the [`Config`]s will be guaranteed to
    /// at least start.
    ///
    /// # Errors
    ///
    /// May return an `Error` if any of the error criteria are met:
    /// 1. Non matching architecture with system
    /// 2. Nonexistent EFI executable if [`BootAction`] is [`BootAction::BootEfi`] or [`BootAction::BootTftp`]
    /// 3. (if applicable) Nonexistent devicetree
    pub fn validate(&self) -> Result<(), ConfigError> {
        self.validate_arch()?;
        self.validate_efi()?;
        self.validate_paths()?;

        Ok(())
    }

    /// Lints a [`Config`], logging a warning if there is something that is wrong
    /// with the [`Config`], but is not fatal.
    pub fn lint(&self) {
        if self.title.as_ref().is_none_or(|x| x.trim().is_empty()) {
            if self.filename.is_empty() {
                warn!(
                    "Config found with no filename or title, assigning a title of its boot index"
                );
            } else {
                warn!("Config {} does not have a title", self.filename);
            }
        }
    }

    /// Picks the preferred title for a [`Config`] using one of three sources.
    ///
    /// If the title of the [`Config`] is found, then that is used and preferred because it indicates the preferred
    /// name for the boot option.
    /// If the title is not present, and the filename is not empty, then the filename is used. This is because it can
    /// still indicate the source of a particular boot option or its origin.
    /// If the filename is empty, then the index of the boot option is used. This is because at least some way of differentiating
    /// the boot option from other boot options is required. This will only be the case if the index is provided.
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_preferred_title(&self, option: Option<usize>) -> String {
        self.title.clone().unwrap_or_else(|| {
            if self.filename.is_empty()
                && let Some(option) = option
            {
                option.to_string()
            } else {
                self.filename.clone()
            }
        })
    }

    /// Validate an architecture by checking if it is the same as the system architecture.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the architecture does not match with the system.
    fn validate_arch(&self) -> Result<(), ConfigError> {
        if let Some(target) = &self.architecture
            && let Some(arch) = get_arch()
            && target != &arch
        {
            return Err(ConfigError::NonMatchingArch((**target).clone()));
        }
        Ok(())
    }

    /// Validate an EFI path by checking if it exists when the [`BootAction`] requires it.
    ///
    /// # Errors
    ///
    /// May return an `Error` if there is no EFI path, and the action field is one of [`BootAction::BootEfi`] or
    /// [`BootAction::BootTftp`].
    fn validate_efi(&self) -> Result<(), ConfigError> {
        if matches!(self.action, BootAction::BootEfi | BootAction::BootTftp)
            && self.efi_path.is_none()
        {
            return Err(ConfigError::ConfigMissingEfi(self.filename.clone()));
        }
        Ok(())
    }

    /// Validates EFI and devicetree paths by checking if it exists within the filesystem.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the paths do not exist in the filesystem when they are in the [`Config`].
    fn validate_paths(&self) -> Result<(), ConfigError> {
        if let Some(handle) = self.fs_handle {
            let mut fs = boot::open_protocol_exclusive(*handle).unwrap_or_else(|_| {
                unreachable!("FsHandle should always support SimpleFileSystem")
            });
            if let Some(efi_path) = &self.efi_path
                && !check_file_exists_str(&mut fs, efi_path).unwrap_or(false)
            {
                return Err(ConfigError::NotExist("EFI", (**efi_path).clone()));
            }
            if let Some(devicetree_path) = &self.devicetree_path
                && !check_file_exists_str(&mut fs, devicetree_path).unwrap_or(false)
            {
                return Err(ConfigError::NotExist(
                    "Devicetree",
                    (**devicetree_path).clone(),
                ));
            }
        } else if self.action == BootAction::BootEfi {
            return Err(ConfigError::ConfigMissingHandle(self.filename.clone()));
        }
        Ok(())
    }
}

/// Gets every [`Config`] from every filesystem that is available, and returns it in a [`Vec<Config>`]
///
/// It will also validate and sort the [`Config`]s.
///
/// # Errors
///
/// May return an `Error` if there are no handles in the system that support [`SimpleFileSystem`].
pub fn scan_configs() -> BootResult<Vec<Config>> {
    let mut configs = Vec::with_capacity(4); // a system is likely to have up to 4 configs
    let handles =
        boot::locate_handle_buffer(SearchType::from_proto::<SimpleFileSystem>())?.to_vec();

    for handle in handles {
        if !is_target_partition(handle) {
            continue;
        }

        let mut fs = boot::open_protocol_exclusive(handle)?;
        parse_all_configs(&mut fs, handle, &mut configs);
    }

    configs.retain_mut(Config::is_good);

    configs.sort_unstable_by(|a, b| {
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

#[cfg(test)]
mod tests {
    use crate::config::types::TypeError;

    use super::*;
    use alloc::borrow::ToOwned;

    // This is technically not a valid Config.
    // This simply tests that the config validator will mark valid fields as correct.
    #[test]
    fn test_non_efi_config() -> Result<(), TypeError> {
        let machine_id = MachineId::new("1234567890abcdef1234567890abcdef")?;
        let sort_key = SortKey::new("linux")?;
        let efi_path = EfiPath::new("\\vmlinuz-linux")?;
        let mut config = Config {
            title: Some("Linux".to_owned()),
            version: Some("6.10.0".to_owned()),
            machine_id: Some(machine_id),
            sort_key: Some(sort_key),
            options: Some("root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv ro".to_owned()),
            efi_path: Some(efi_path),
            filename: "linux.conf".to_owned(),
            suffix: ".conf".to_owned(),
            action: BootAction::BootTftp,
            ..Config::default()
        };
        assert!(config.is_good());
        Ok(())
    }
}

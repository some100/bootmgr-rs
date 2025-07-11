//! Provides [`Config`], the main configuration struct

use alloc::{string::String, vec, vec::Vec};
use log::{error, warn};
use uefi::{
    Handle,
    boot::{self, SearchType},
    proto::media::fs::SimpleFileSystem,
};

use crate::{
    boot::action::BootAction,
    config::parsers::parse_all_configs,
    error::BootError,
    system::{
        fs::{check_file_exists_str, check_path_valid, is_target_partition},
        helper::get_arch,
    },
};

pub mod builder;
pub mod parsers;

const MACHINE_ID_LEN: usize = 32;

/// The standard [`Config`]
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub title: Option<String>,
    pub version: Option<String>,
    pub machine_id: Option<String>,
    pub sort_key: Option<String>,
    pub options: Option<String>,
    pub devicetree: Option<String>,
    pub architecture: Option<String>,

    pub action: BootAction,
    pub bad: bool,
    pub handle: Option<Handle>,
    pub efi: String,
    pub filename: String,
    pub suffix: String,
}

impl Config {
    /// Returns a [`Vec`] over every [`String`] struct field that should be edited
    #[must_use]
    pub fn get_str_fields(&self) -> Vec<(&'static str, Option<&String>)> {
        vec![
            ("title", self.title.as_ref()),
            ("version", self.version.as_ref()),
            ("machine_id", self.machine_id.as_ref()),
            ("sort_key", self.sort_key.as_ref()),
            ("options", self.options.as_ref()),
            ("devicetree", self.devicetree.as_ref()),
            ("architecture", self.architecture.as_ref()),
            ("efi", Some(&self.efi)),
        ]
    }

    /// Verifies if a [`Config`] is good. If the [`Config`] is good, then
    /// it will return true. Otherwise, it will return `false`.
    pub fn is_good(&mut self) -> bool {
        self.lint();
        if let Err(e) = self.validate() {
            error!("{e}");
            return false;
        }
        true
    }

    /// Validates a [`Config`], returning an `Error` if any of the "fail" criteria
    /// are met. This ensures that any of the [`Config`]s will be guaranteed to
    /// at least start.
    ///
    /// # Errors
    ///
    /// May return an `Error` if any of the error criteria are met:
    /// 1. Non matching architecture with system
    /// 2. Invalid sort key
    /// 3. Invalid machine ID
    /// 4. Invalid EFI executable path
    /// 5. Invalid devicetree path
    /// 6. Nonexistent EFI executable
    /// 7. (if applicable) Nonexistent devicetree
    pub fn validate(&mut self) -> Result<(), BootError> {
        if let Some(target) = &self.architecture
            && let Some(arch) = get_arch()
            && *target != arch
        {
            return Err(BootError::NonMatchingArch(target.clone()));
        }
        if let Some(sort_key) = &self.sort_key
            && !sort_key
                .chars()
                .all(|x| x.is_ascii_alphanumeric() || x == '.' || x == '_' || x == '-')
        {
            error!("Config {} has invalid sort key {}", self.filename, sort_key);
            self.sort_key = None;
        }
        if let Some(machine_id) = &self.machine_id
            && (machine_id.chars().count() != MACHINE_ID_LEN
                || !machine_id
                    .chars()
                    .all(|x| x.is_ascii_hexdigit() && x.is_ascii_lowercase()))
        {
            error!(
                "Config {} has invalid machine id {}",
                self.filename, machine_id
            );
            self.machine_id = None;
        }
        if !check_path_valid(&self.efi) {
            return Err(BootError::InvalidPath(
                self.filename.clone(),
                self.efi.clone(),
            ));
        }
        if let Some(devicetree) = &self.devicetree
            && !check_path_valid(devicetree)
        {
            return Err(BootError::InvalidPath(
                self.filename.clone(),
                devicetree.clone(),
            ));
        }
        if let Some(handle) = self.handle {
            let mut fs = boot::open_protocol_exclusive(handle)?;
            match check_file_exists_str(&mut fs, &self.efi) {
                Ok(false) | Err(_) => return Err(BootError::NotExist("EFI", self.efi.clone())),
                _ => (),
            }
            if let Some(devicetree) = &self.devicetree {
                match check_file_exists_str(&mut fs, devicetree) {
                    Ok(false) | Err(_) => {
                        return Err(BootError::NotExist("Devicetree", devicetree.clone()));
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    /// Lints a [`Config`], logging a warning if there is something that is wrong
    /// with the [`Config`], but is not fatal.
    pub fn lint(&self) {
        if self.title.as_ref().is_none_or(|x| x.trim().is_empty()) {
            warn!("Config {} does not have a title", self.filename);
        }
    }
}

/// Gets every [`Config`] from every filesystem that is available, and returns it in a [`Vec<Config>`]
///
/// It will also validate and sort the [`Config`]s.
///
/// # Errors
///
/// May return an `Error` if there are no handles in the system that support [`SimpleFileSystem`].
pub fn get_configs() -> uefi::Result<Vec<Config>> {
    let mut configs = Vec::with_capacity(4); // a system is likely to have up to 4 configs
    let handles =
        boot::locate_handle_buffer(SearchType::from_proto::<SimpleFileSystem>())?.to_vec();

    for handle in handles {
        if !is_target_partition(&handle) {
            continue;
        }

        let mut fs = boot::open_protocol_exclusive(handle)?;
        parse_all_configs(&mut fs, handle, &mut configs);
    }

    configs.retain_mut(Config::is_good);

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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;

    #[test]
    fn test_full_config() {
        let mut config = Config {
            title: Some("Linux".to_owned()),
            version: Some("6.10.0".to_owned()),
            machine_id: Some("1234567890abcdefghijklmnopqrstuv".to_owned()),
            sort_key: Some("linux".to_owned()),
            options: Some("root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv ro".to_owned()),
            efi: "\\vmlinuz-linux".to_owned(),
            filename: "linux.conf".to_owned(),
            suffix: ".conf".to_owned(),
            ..Config::default()
        };
        assert!(config.is_good());
    }

    #[test]
    fn test_invalid_sort_key() {
        let mut config = Config {
            sort_key: Some(";'[];\\[]-=invalid sort key".to_owned()),
            efi: "\\foo\\bar".to_owned(),
            ..Config::default()
        };
        config.validate().expect("Config is invalid outside of sort key"); // this means that the validate function is borked
        assert!(config.sort_key.is_none());
    }

    #[test]
    fn test_invalid_machine_id() {
        let mut config = Config {
            machine_id: Some("invalidthing".to_owned()), // obviously invalid
            efi: "\\foo\\bar".to_owned(),
            ..Config::default()
        };
        config.validate().expect("Config is invalid outside of machine id");
        assert!(config.machine_id.is_none());
        config.machine_id = Some("1".to_owned());
        config.validate().expect("Config is invalid outside of machine id");
        assert!(config.machine_id.is_none());
        config.machine_id = Some("1234567890abcdefghijklmnopqrstu".to_owned()); // slightly less obviously invalid
        config.validate().expect("Config is invalid outside of machine id");
        assert!(config.machine_id.is_none());
    }

    #[test]
    fn test_invalid_efi_path() {
        let mut config = Config {
            efi: "** -= . : <> ? \\very very bad path".to_owned(),
            ..Config::default()
        };
        assert!(!config.is_good());
        let mut config = Config {
            efi: "/a/path/with/forward/slashes".to_owned(),
            ..Config::default()
        };
        assert!(!config.is_good());
    }

    #[test]
    fn test_invalid_dtb_path() {
        let mut config = Config {
            devicetree: Some("\\** / : ???? .dtb".to_owned()),
            efi: "\\foo\\bar".to_owned(),
            ..Config::default()
        };
        assert!(!config.is_good());
    }
}

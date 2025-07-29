//! Configuration builder.

use alloc::string::String;
use log::warn;
use uefi::Handle;

use crate::{
    boot::action::BootAction,
    config::{
        Config,
        types::{Architecture, DevicetreePath, EfiPath, FsHandle, MachineId, SortKey},
    },
};

/// A builder to configure a [`Config`]
///
/// # Example
///
/// ```no_run
/// let config = ConfigBuilder::new("\\EFI\\BOOT\\BOOTx64.efi", "foo.conf", ".conf")
///     .title("foo")
///     .handle(uefi::boot::image_handle())
///     .build();
/// ```
#[must_use = "Has no effect if the result is unused"]
pub struct ConfigBuilder {
    /// The inner [`Config`] that the builder operates on.
    pub config: Config,
}

impl ConfigBuilder {
    /// Constructs a new [`Config`].
    pub fn new(filename: impl Into<String>, suffix: impl Into<String>) -> Self {
        let filename = filename.into();
        let suffix = suffix.into();
        Self {
            config: Config {
                title: None,
                version: None,
                machine_id: None,
                sort_key: None,
                options: None,
                devicetree: None,
                architecture: None,
                efi: None,
                bad: false,
                action: BootAction::BootEfi,
                handle: None,
                filename,
                suffix,
            },
        }
    }

    /// Sets the title of a [`Config`].
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.config.title = Some(title.into());
        self
    }

    /// Sets the version of a [`Config`].
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.config.version = Some(version.into());
        self
    }

    /// Sets the machine id of a [`Config`].
    ///
    /// This must be formatted as 32 lower case hexadecimal characters as defined in
    /// `BootLoaderSpec`. Otherwise, this will have no effect
    pub fn machine_id(mut self, machine_id: impl Into<String>) -> Self {
        self.config.machine_id = match MachineId::new(&machine_id.into()) {
            Ok(machine_id) => Some(machine_id),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets the sort key of a [`Config`]
    ///
    /// Ideally, this should be entirely composed of lowercase characters,
    /// with nothing else other than numbers, dashes, underscores, and periods.
    pub fn sort_key(mut self, sort_key: impl Into<String>) -> Self {
        self.config.sort_key = match SortKey::new(&sort_key.into()) {
            Ok(sort_key) => Some(sort_key),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets the options of a [`Config`]
    ///
    /// This essentially sets the `LoadOptions`, or the command line of an EFI shell
    pub fn options(mut self, options: impl Into<String>) -> Self {
        self.config.options = Some(options.into());
        self
    }

    /// Sets the devicetree of a [`Config`]
    pub fn devicetree(mut self, devicetree: impl Into<String>) -> Self {
        self.config.devicetree = match DevicetreePath::new(&devicetree.into()) {
            Ok(devicetree) => Some(devicetree),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets the architecture of a [`Config`]
    ///
    /// This is only used for filtering entries
    pub fn architecture(mut self, architecture: impl Into<String>) -> Self {
        self.config.architecture = match Architecture::new(&architecture.into()) {
            Ok(architecture) => Some(architecture),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets if a [`Config`] is bad, so it may be deranked
    pub const fn bad(mut self, bad: bool) -> Self {
        self.config.bad = bad;
        self
    }

    /// Sets the [`BootAction`] of a [`Config`]
    ///
    /// This can be one of [`BootAction::BootEfi`], [`BootAction::BootTftp`], [`BootAction::Reboot`], [`BootAction::Shutdown`],
    /// and [`BootAction::ResetToFirmware`]. You should never need to use this
    pub const fn action(mut self, action: BootAction) -> Self {
        self.config.action = action;
        self
    }

    /// Sets the [`Handle`] of a [`Config`]
    ///
    /// This is used for filesystem operations, so it is required to be set to
    /// indicate which filesystem a [`Config`] comes from
    pub fn handle(mut self, handle: Handle) -> Self {
        self.config.handle = match FsHandle::new(handle) {
            Ok(handle) => Some(handle),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets the EFI executable path of a [`Config`].
    pub fn efi(mut self, efi: impl Into<String>) -> Self {
        self.config.efi = match EfiPath::new(&efi.into()) {
            Ok(efi) => Some(efi),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Builds a [`Config`]
    #[must_use = "Has no effect if the result is unused"]
    pub fn build(self) -> Config {
        self.config
    }
}

impl From<Config> for ConfigBuilder {
    fn from(config: Config) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;

    #[test]
    fn test_basic_config() {
        let config = ConfigBuilder::new("foo.bar", ".bar")
            .efi("\\foo\\foo.bar")
            .title("Some title")
            .version("some.version")
            .sort_key("some-sort-key")
            .options("Some options")
            .build();

        assert_eq!(*config.efi.unwrap(), "\\foo\\foo.bar".to_owned());
        assert_eq!(config.filename, "foo.bar".to_owned());
        assert_eq!(config.suffix, ".bar".to_owned());
        assert_eq!(config.title, Some("Some title".to_owned()));
        assert_eq!(config.version, Some("some.version".to_owned()));
        assert_eq!(*config.sort_key.unwrap(), "some-sort-key");
        assert_eq!(config.options, Some("Some options".to_owned()));
    }

    #[test]
    fn test_path_replacement() {
        let config = ConfigBuilder::new("foo.bar", ".bar")
            .efi("/foo/foo.bar")
            .devicetree("/baz/baz.qux")
            .build();

        assert_eq!(*config.efi.unwrap(), "\\foo\\foo.bar".to_owned());
        assert_eq!(*config.devicetree.unwrap(), "\\baz\\baz.qux".to_owned());
    }
}

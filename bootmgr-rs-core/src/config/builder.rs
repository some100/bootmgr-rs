//! Configuration builder.

use alloc::string::String;
use log::warn;
use uefi::Handle;

use crate::{
    boot::action::BootAction,
    config::{
        Config,
        parsers::Parsers,
        types::{Architecture, DevicetreePath, EfiPath, FsHandle, MachineId, SortKey},
    },
};

/// A builder to configure a [`Config`]
///
/// # Example
///
/// ```no_run
/// use bootmgr_rs_core::config::builder::ConfigBuilder;
/// use uefi::{boot, proto::{device_path::DevicePath, loaded_image::LoadedImage, media::fs::SimpleFileSystem}};
///
/// let handle = {
///     let loaded_image =
///         boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).expect("Failed to open LoadedImage on image");
///     let device_handle = loaded_image.device().expect("Image was not loaded from a filesystem");
///     let device_path = boot::open_protocol_exclusive::<DevicePath>(device_handle).expect("Failed to get device path from image filesystem");
///     boot::locate_device_path::<SimpleFileSystem>(&mut &*device_path).expect("Failed to get SimpleFileSystem on image filesystem")
/// };
///
/// let config = ConfigBuilder::new("foo.conf", ".conf")
///     .title("foo")
///     .fs_handle(handle)
///     .build();
/// ```
#[must_use = "Has no effect if the result is unused"]
pub struct ConfigBuilder {
    /// The inner [`Config`] that the builder operates on.
    config: Config,
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
                devicetree_path: None,
                architecture: None,
                efi_path: None,
                bad: false,
                action: BootAction::BootEfi,
                fs_handle: None,
                origin: None,
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
    pub fn devicetree_path(mut self, devicetree_path: impl Into<String>) -> Self {
        self.config.devicetree_path = match DevicetreePath::new(&devicetree_path.into()) {
            Ok(devicetree_path) => Some(devicetree_path),
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
    pub const fn set_bad(mut self, bad: bool) -> Self {
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
    pub fn fs_handle(mut self, fs_handle: Handle) -> Self {
        self.config.fs_handle = match FsHandle::new(fs_handle) {
            Ok(fs_handle) => Some(fs_handle),
            Err(e) => {
                warn!("{e}");
                None
            }
        };
        self
    }

    /// Sets the origin of a [`Config`].
    ///
    /// This is one of the parsers that generate [`Config`]s.
    pub const fn origin(mut self, origin: Parsers) -> Self {
        self.config.origin = Some(origin);
        self
    }

    /// Sets the EFI executable path of a [`Config`].
    pub fn efi_path(mut self, efi_path: impl Into<String>) -> Self {
        self.config.efi_path = match EfiPath::new(&efi_path.into()) {
            Ok(efi_path) => Some(efi_path),
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

    /// Assigns a value to a field in a [`Config`] if it is [`Some`].
    pub fn assign_if_some<F, T>(self, value: Option<T>, assign: F) -> Self
    where
        F: FnOnce(Self, T) -> Self,
    {
        if let Some(value) = value {
            assign(self, value)
        } else {
            self
        }
    }
}

impl From<&Config> for ConfigBuilder {
    fn from(value: &Config) -> Self {
        Self::new(&value.filename, &value.suffix)
            .set_bad(value.bad)
            .assign_if_some(value.title.as_ref(), Self::title)
            .assign_if_some(value.version.as_ref(), Self::version)
            .assign_if_some(value.machine_id.as_deref(), Self::machine_id)
            .assign_if_some(value.sort_key.as_deref(), Self::sort_key)
            .assign_if_some(value.options.as_ref(), Self::options)
            .assign_if_some(value.devicetree_path.as_deref(), Self::devicetree_path)
            .assign_if_some(value.architecture.as_deref(), Self::architecture)
            .assign_if_some(value.efi_path.as_deref(), Self::efi_path)
            .assign_if_some(value.fs_handle.as_deref().copied(), Self::fs_handle)
            .assign_if_some(value.origin, Self::origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::ToOwned;

    #[test]
    fn test_basic_config() {
        let config = ConfigBuilder::new("foo.bar", ".bar")
            .efi_path("\\foo\\foo.bar")
            .title("Some title")
            .version("some.version")
            .sort_key("some-sort-key")
            .options("Some options")
            .build();

        assert_eq!(
            config.efi_path.as_deref(),
            Some(&"\\foo\\foo.bar".to_owned())
        );
        assert_eq!(config.filename, "foo.bar".to_owned());
        assert_eq!(config.suffix, ".bar".to_owned());
        assert_eq!(config.title, Some("Some title".to_owned()));
        assert_eq!(config.version, Some("some.version".to_owned()));
        assert_eq!(
            config.sort_key.as_deref(),
            Some(&"some-sort-key".to_owned())
        );
        assert_eq!(config.options, Some("Some options".to_owned()));
    }

    #[test]
    fn test_path_replacement() {
        let config = ConfigBuilder::new("foo.bar", ".bar")
            .efi_path("/foo/foo.bar")
            .devicetree_path("/baz/baz.qux")
            .build();

        assert_eq!(
            config.efi_path.as_deref(),
            Some(&"\\foo\\foo.bar".to_owned())
        );
        assert_eq!(
            config.devicetree_path.as_deref(),
            Some(&"\\baz\\baz.qux".to_owned())
        );
    }
}

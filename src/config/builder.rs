//! Configuration builder

use alloc::string::String;
use uefi::Handle;

use crate::{boot::action::BootAction, config::Config, system::helper::normalize_path};

/// A builder to configure a [`Config`]
///
/// # Example
///
/// ```
/// let config = ConfigBuilder::new("\\EFI\\BOOT\\BOOTx64.efi", "foo.conf", ".conf")
///     .title("foo")
///     .handle(uefi::boot::image_handle())
///     .build();
/// ```
#[must_use]
pub struct ConfigBuilder {
    pub config: Config,
}

impl ConfigBuilder {
    /// Constructs a new [`Config`].
    pub fn new(
        efi: impl Into<String>,
        filename: impl Into<String>,
        suffix: impl Into<String>,
    ) -> Self {
        let efi = normalize_path(&efi.into()); // replace any forward slashes with backslashes if any exist
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
                bad: false,
                action: BootAction::Boot,
                handle: None,
                efi,
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
        self.config.machine_id = Some(machine_id.into());
        self
    }

    /// Sets the sort key of a [`Config`]
    ///
    /// Ideally, this should be entirely composed of lowercase characters,
    /// with nothing else other than numbers, dashes, underscores, and periods.
    pub fn sort_key(mut self, sort_key: impl Into<String>) -> Self {
        self.config.sort_key = Some(sort_key.into());
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
        self.config.devicetree = Some(devicetree.into());
        self
    }

    /// Sets the architecture of a [`Config`]
    ///
    /// This is only used for filtering entries
    pub fn architecture(mut self, architecture: impl Into<String>) -> Self {
        self.config.architecture = Some(architecture.into());
        self
    }

    /// Sets if a [`Config`] is bad, so it may be deranked
    pub fn bad(mut self, bad: bool) -> Self {
        self.config.bad = bad;
        self
    }

    /// Sets the [`BootAction`] of a [`Config`]
    ///
    /// This can be one of [`BootAction::Boot`], [`BootAction::Reboot`], [`BootAction::Shutdown`],
    /// and [`BootAction::ResetToFirmware`]. You should never need to use this
    pub fn action(mut self, action: BootAction) -> Self {
        self.config.action = action;
        self
    }

    /// Sets the [`Handle`] of a [`Config`]
    ///
    /// This is used for filesystem operations, so it is required to be set to
    /// indicate which filesystem a [`Config`] comes from
    pub fn handle(mut self, handle: Handle) -> Self {
        self.config.handle = Some(handle);
        self
    }

    /// Builds a [`Config`]
    #[must_use]
    pub fn build(self) -> Config {
        self.config
    }
}

//! A persistent [`Config`] overlay.

use alloc::{borrow::ToOwned, collections::btree_map::BTreeMap, string::String};
use bootmgr_rs_core::{
    BootResult,
    config::{Config, builder::ConfigBuilder, parsers::Parsers},
    system::fs::{create, read, write},
};
use serde::{Deserialize, Serialize};
use uefi::{CStr16, boot, cstr16};

/// The path where the persistent [`Config`]s are stored.
const PERSISTENT_CONFIG_PATH: &CStr16 = cstr16!("\\loader\\bootmgr-rs-saved.conf");

/// The editable fields of a [`Config`] that may be serialized into a persistent file.
#[derive(Serialize, Deserialize)]
struct SerializableConfig {
    /// The title of the configuration.
    title: Option<String>,

    /// The version of the configuration.
    version: Option<String>,

    /// The machine-id of the configuration.
    machine_id: Option<String>,

    /// The sort-key of the configuration.
    sort_key: Option<String>,

    /// The boot options of the configuration.
    options: Option<String>,

    /// The devicetree path of the configuration.
    devicetree_path: Option<String>,

    /// The architecture of the configuration.
    architecture: Option<String>,

    /// The efi path of the configuration.
    efi_path: Option<String>,

    /// The origin of the configuration (should not be changed).
    origin: Option<String>,
}

impl From<Config> for SerializableConfig {
    fn from(value: Config) -> Self {
        Self {
            title: value.title,
            version: value.version,
            machine_id: value.machine_id.as_deref().cloned(),
            sort_key: value.sort_key.as_deref().cloned(),
            options: value.options,
            devicetree_path: value.devicetree_path.as_deref().cloned(),
            architecture: value.architecture.as_deref().cloned(),
            efi_path: value.efi_path.as_deref().cloned(),
            origin: value.origin.map(|x| x.as_str().into()),
        }
    }
}

/// The main storage for persistent [`Config`]s. This is essentially
/// a map of filenames to a saved [`Config`].
#[derive(Default)]
pub struct PersistentConfig {
    /// The mapper for config filenames to [`Config`].
    configs: BTreeMap<String, SerializableConfig>,
}

impl PersistentConfig {
    /// Create a new [`PersistentConfig`].
    ///
    /// This will essentially read from the saved config path, deserialize each line into a [`Config`], then
    /// add that [`Config`] to the persistent config storage.
    pub fn new() -> BootResult<Self> {
        let mut configs = Self::default();

        let mut fs = boot::get_image_file_system(boot::image_handle())?;
        if let Ok(content) = read(&mut fs, PERSISTENT_CONFIG_PATH)
            && let Ok(content) = postcard::from_bytes(&content)
        {
            configs.configs = content;
        }
        Ok(configs)
    }

    /// Check if the [`PersistentConfig`] contains a certain entry.
    /// 
    /// This compares the filename and origin of the [`Config`]s. If the filename
    /// and origin are both exactly the same, then it is most likely the same [`Config`].
    pub fn contains(&self, config: &Config) -> bool {
        self.configs
            .get(&config.filename)
            .map(|x| x.origin.as_ref())
            == Some(config.origin.map(|x| x.as_str().to_owned()).as_ref())
    }

    /// Save the [`Config`]s in the [`PersistentConfig`] to the filesystem.
    pub fn save_to_fs(&self) -> BootResult<()> {
        let mut fs = boot::get_image_file_system(boot::image_handle())?;
        create(&mut fs, PERSISTENT_CONFIG_PATH)?;

        if let Ok(content) = postcard::to_allocvec(&self.configs) {
            write(&mut fs, PERSISTENT_CONFIG_PATH, &content)?;
        }
        Ok(())
    }

    /// Optionally swap a mutable [`Config`] with one that is stored in the [`PersistentConfig`].
    ///
    /// This will only swap the 8 fields that the editor is able to edit.
    pub fn swap_config_in_persist<'a>(&'a self, config: &'a mut Config) {
        if let Some(persist_config) = self.configs.get(&config.filename)
            && persist_config.origin.as_deref() == config.origin.map(Parsers::as_str)
        {
            *config = ConfigBuilder::from(&*config)
                .assign_if_some(persist_config.title.as_ref(), ConfigBuilder::title)
                .assign_if_some(persist_config.version.as_ref(), ConfigBuilder::version)
                .assign_if_some(
                    persist_config.machine_id.as_deref(),
                    ConfigBuilder::machine_id,
                )
                .assign_if_some(persist_config.sort_key.as_deref(), ConfigBuilder::sort_key)
                .assign_if_some(persist_config.options.as_ref(), ConfigBuilder::options)
                .assign_if_some(
                    persist_config.devicetree_path.as_deref(),
                    ConfigBuilder::devicetree_path,
                )
                .assign_if_some(
                    persist_config.architecture.as_deref(),
                    ConfigBuilder::architecture,
                )
                .assign_if_some(persist_config.efi_path.as_deref(), ConfigBuilder::efi_path)
                .build();
        }
    }

    /// Add a [`Config`] into the [`PersistentConfig`] map.
    pub fn add_config_to_persist(&mut self, config: &Config) {
        self.configs
            .insert(config.filename.clone(), config.clone().into());
    }

    /// Remove a [`Config`] from the [`PersistentConfig`] map.
    pub fn remove_config_from_persist(&mut self, config: &Config) {
        self.configs.remove(&config.filename);
    }
}

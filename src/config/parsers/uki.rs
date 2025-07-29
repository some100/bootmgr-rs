#![cfg(feature = "uki")]

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;

use object::{Object, ObjectSection, Section};

use thiserror::Error;
use uefi::{
    CStr16, Handle,
    boot::ScopedProtocol,
    cstr16,
    proto::media::{file::FileInfo, fs::SimpleFileSystem},
};

use crate::{
    BootResult,
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{
        fs::{read, read_filtered_dir},
        helper::get_path_cstr,
    },
};

const UKI_PREFIX: &CStr16 = cstr16!("\\EFI\\Linux");
const UKI_SUFFIX: &str = ".efi";

/// Errors that may result from parsing the UKI config.
#[derive(Error, Debug)]
pub enum UkiError {
    /// An error that originated from the [`object`] crate.
    #[error("Error while parsing PE binary: {0}")]
    Object(#[from] object::Error),
}

#[derive(Default)]
struct Osrel {
    name: Option<String>,
    id: Option<String>,
    image_id: Option<String>,
    image_version: Option<String>,
    pretty_name: Option<String>,
    version: Option<String>,
    version_id: Option<String>,
    build_id: Option<String>,
}

impl Osrel {
    fn new(content: Option<Section<'_, '_>>) -> Result<Self, UkiError> {
        let mut osrel = Self::default();
        if let Some(content) = content {
            let content_bytes = content.data()?;
            let content_str = String::from_utf8_lossy(content_bytes).replace('"', "");

            for line in content_str.lines() {
                let line = line.trim();
                if let Some((key, value)) = line.split_once('=') {
                    let value = value.trim().to_owned();
                    match key {
                        "NAME" => osrel.name = Some(value),
                        "ID" => osrel.id = Some(value),
                        "IMAGE_ID" => osrel.image_id = Some(value),
                        "IMAGE_VERSION" => osrel.image_version = Some(value),
                        "PRETTY_NAME" => osrel.pretty_name = Some(value),
                        "VERSION" => osrel.version = Some(value),
                        "VERSION_ID" => osrel.version_id = Some(value),
                        "BUILD_ID" => osrel.build_id = Some(value),
                        _ => (),
                    }
                }
            }
        }
        Ok(osrel)
    }
}

/// The parser for UKIs (also known as `BootLoaderSpec` type #2 files)
pub struct UkiConfig {
    title: String,
    sort_key: String,
    version: Option<String>,
}

impl UkiConfig {
    /// # Errors
    ///
    /// May return an `Error` if the provided content is not a PE file.
    pub fn new(content: &[u8]) -> Result<Self, UkiError> {
        let pe = object::File::parse(content)?;
        let section = pe.section_by_name(".osrel");

        let osrel = match Osrel::new(section) {
            Ok(osrel) => osrel,
            Err(e) => {
                warn!("{e}"); // the section might not exist, so according to the spec just use the defaults
                Osrel::default()
            }
        };

        Ok(Self {
            title: osrel
                .pretty_name
                .clone()
                .or_else(|| osrel.image_id.clone())
                .or_else(|| osrel.name.clone())
                .or_else(|| osrel.id.clone())
                .unwrap_or_else(|| "Linux".to_owned()), // we preferably want pretty name, but title works too
            sort_key: osrel
                .image_id
                .or(osrel.id)
                .unwrap_or_else(|| "linux".to_owned()),
            version: osrel
                .image_version
                .or(osrel.version)
                .or(osrel.version_id)
                .or(osrel.build_id),
        })
    }
}

impl ConfigParser for UkiConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let dir = read_filtered_dir(fs, UKI_PREFIX, UKI_SUFFIX);

        for file in dir {
            match get_uki_config(&file, fs, handle) {
                Ok(config) => configs.push(config),
                Err(e) => warn!("{e}"),
            }
        }
    }
}

fn get_uki_config(
    file: &FileInfo,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
) -> BootResult<Config> {
    let content = read(fs, &get_path_cstr(UKI_PREFIX, file.file_name())?)?;

    let uki_config = UkiConfig::new(&content)?;

    let efi = format!("{UKI_PREFIX}\\{}", file.file_name());
    let mut config = ConfigBuilder::new(file.file_name(), UKI_SUFFIX)
        .efi(efi)
        .title(uki_config.title)
        .sort_key(uki_config.sort_key)
        .handle(handle);

    if let Some(version) = uki_config.version {
        config = config.version(version);
    }

    Ok(config.build())
}

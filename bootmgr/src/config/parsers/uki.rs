// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! A parser for BootLoaderSpec type #2, a versionless specification for single Linux boot binaries.

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

use log::warn;
use object::{Architecture, Object, ObjectSection, Section};
use thiserror::Error;
use uefi::{CStr16, Handle, Status, cstr16, proto::media::file::FileInfo};

use crate::{
    BootResult,
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    system::{
        fs::{FsError, UefiFileSystem},
        helper::get_path_cstr,
    },
};

/// The configuration prefix.
const UKI_PREFIX: &CStr16 = cstr16!("\\EFI\\Linux");

/// The configuration suffix.
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
    /// The `NAME` specified in .osrel
    name: Option<String>,

    /// The `ID` specified in .osrel
    id: Option<String>,

    /// The `IMAGE_ID` specified in .osrel
    image_id: Option<String>,

    /// The `IMAGE_VERSION` specified in .osrel
    image_version: Option<String>,

    /// The `PRETTY_NAME` specified in .osrel
    pretty_name: Option<String>,

    /// The `VERSION` specified in .osrel
    version: Option<String>,

    /// The `VERSION_ID` specified in .osrel
    version_id: Option<String>,

    /// The `BUILD_ID` specified in .osrel
    build_id: Option<String>,
}

impl Osrel {
    /// Create a new [`Osrel`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the section does not contain any data.
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
    /// The title of the configuration.
    title: String,

    /// The sort-key of the configuration.
    sort_key: String,

    /// The version of the configuration.
    version: Option<String>,

    /// The architecture of the configuration.
    architecture: Option<String>,
}

impl UkiConfig {
    /// Creates a new [`UkiConfig`].
    ///
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

        let architecture = match pe.architecture() {
            Architecture::X86_64 => Some("x64"),
            Architecture::I386 => Some("x86"),
            Architecture::Aarch64 => Some("aa64"),
            Architecture::Arm => Some("arm"),
            _ => None,
        }
        .map(ToOwned::to_owned); // this will clone anyways when added to the builder.

        Ok(Self {
            title: osrel
                .pretty_name
                .as_ref()
                .or(osrel.image_id.as_ref())
                .or(osrel.name.as_ref())
                .or(osrel.id.as_ref())
                .map_or("Linux", |v| v)
                .to_owned(),
            sort_key: osrel
                .image_id
                .as_ref()
                .or(osrel.id.as_ref())
                .map_or("linux", |v| v)
                .to_owned(),
            version: osrel
                .image_version
                .or(osrel.version)
                .or(osrel.version_id)
                .or(osrel.build_id),
            architecture,
        })
    }
}

impl ConfigParser for UkiConfig {
    fn parse_configs(fs: &mut UefiFileSystem, handle: Handle, configs: &mut Vec<Config>) {
        let dir = fs.read_filtered_dir(UKI_PREFIX, UKI_SUFFIX);

        for file in dir {
            match get_uki_config(&file, fs, handle) {
                Ok(Some(config)) => configs.push(config),
                Err(e) => warn!("{e}"),
                _ => (),
            }
        }
    }
}

/// Parse a UKI executable given the [`FileInfo`], a `SimpleFileSystem` protocol, and a handle to that protocol.
///
/// # Errors
///
/// May return an `Error` if the
fn get_uki_config(
    file: &FileInfo,
    fs: &mut UefiFileSystem,
    handle: Handle,
) -> BootResult<Option<Config>> {
    let content = match fs.read(&get_path_cstr(UKI_PREFIX, file.file_name())?) {
        Ok(content) => content,
        Err(FsError::OpenErr(Status::NOT_FOUND)) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let uki_config = UkiConfig::new(&content)?;

    let efi_path = format!("{UKI_PREFIX}\\{}", file.file_name());
    let config = ConfigBuilder::new(file.file_name(), UKI_SUFFIX)
        .efi_path(efi_path)
        .title(uki_config.title)
        .sort_key(uki_config.sort_key)
        .fs_handle(handle)
        .origin(Parsers::Uki)
        .assign_if_some(uki_config.version, ConfigBuilder::version)
        .assign_if_some(uki_config.architecture, ConfigBuilder::architecture);

    Ok(Some(config.build()))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #[test]
        fn doesnt_panic(x in any::<Vec<u8>>()) {
            let _ = UkiConfig::new(&x);
        }
    }
}

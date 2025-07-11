#![cfg(feature = "uki")]

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;

#[cfg(target_pointer_width = "64")]
use pelite::pe64 as pe;

#[cfg(target_pointer_width = "32")]
use pelite::pe32 as pe;

use pe::{Pe, PeFile, headers::SectionHeader};

use uefi::{
    CStr16, Handle,
    boot::ScopedProtocol,
    cstr16,
    proto::media::{file::FileInfo, fs::SimpleFileSystem},
};

use crate::{
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    error::BootError,
    system::{
        fs::{read, read_filtered_dir},
        helper::get_path_cstr,
    },
};

const UKI_PREFIX: &CStr16 = cstr16!("\\EFI\\Linux");
const UKI_SUFFIX: &str = ".efi";

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
    fn new(content: Option<&SectionHeader>, view: &PeFile) -> Result<Self, BootError> {
        let mut osrel = Osrel::default();
        if let Some(content) = content {
            let content_bytes = match view.get_section_bytes(content) {
                Ok(content_bytes) => content_bytes,
                Err(e) => return Err(BootError::Pe(e)),
            };
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
    pub fn new(content: &[u8]) -> Result<Self, BootError> {
        let pefile = pe::PeFile::from_bytes(content).map_err(BootError::Pe)?;

        // may cause a panic due to passing a pointer that is unaligned or null with some malformed inputs.
        // this seems to be a bug with pelite
        let sections = pefile.section_headers();

        let osrel = match Osrel::new(sections.by_name(".osrel"), &pefile) {
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
                .or(osrel.image_id.clone())
                .or(osrel.name.clone())
                .or(osrel.id.clone())
                .unwrap_or("Linux".to_owned()), // we preferably want pretty name, but title works too
            sort_key: osrel.image_id.or(osrel.id).unwrap_or("linux".to_owned()),
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
) -> Result<Config, BootError> {
    let content = read(fs, &get_path_cstr(UKI_PREFIX, file.file_name()))?;

    let uki_config = UkiConfig::new(&content)?;

    let efi = format!("{UKI_PREFIX}\\{}", file.file_name());
    let mut config = ConfigBuilder::new(efi, file.file_name(), UKI_SUFFIX)
        .title(uki_config.title)
        .sort_key(uki_config.sort_key)
        .handle(handle);

    if let Some(version) = uki_config.version {
        config = config.version(version);
    }

    Ok(config.build())
}

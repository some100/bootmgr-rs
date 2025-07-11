use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;

#[cfg(target_pointer_width = "64")]
use pelite::pe64 as pe;

#[cfg(target_pointer_width = "32")]
use pelite::pe32 as pe;

use pe::{Pe, PeFile, headers::SectionHeader};

use uefi::{CStr16, Handle, cstr16, fs::FileSystem, proto::media::file::FileInfo};

use crate::{
    error::BootError,
    parsers::{Config, ConfigParser},
    system::helper::{get_path_cstr, read_filtered_dir},
};

const UKI_PREFIX: &CStr16 = cstr16!("\\EFI\\Linux");

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
            let content_str = String::from_utf8_lossy(content_bytes).replace("\"", "");

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

impl Default for Osrel {
    fn default() -> Self {
        Self {
            name: None,
            id: None,
            image_id: None,
            image_version: None,
            pretty_name: None,
            version: None,
            version_id: None,
            build_id: None,
        }
    }
}

pub struct UkiConfig {
    title: String,
    sort_key: String,
    version: Option<String>,
}

impl UkiConfig {
    fn new(content: &[u8]) -> Result<Self, BootError> {
        let pefile = pe::PeFile::from_bytes(content).map_err(|x| BootError::Pe(x))?;
        let sections = pefile.section_headers();

        let osrel = match Osrel::new(sections.by_name(".osrel"), &pefile) {
            Ok(osrel) => osrel,
            Err(e) => {
                warn!("{e}");
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
    fn parse_configs(fs: &mut FileSystem, handle: &Handle, configs: &mut Vec<Config>) {
        let dir = read_filtered_dir(fs, UKI_PREFIX, ".efi");

        for file in dir {
            if let Some(config) = get_uki_config(&file, fs, handle) {
                configs.push(config);
            }
        }
    }
}

fn get_uki_config(file: &FileInfo, fs: &mut FileSystem, handle: &Handle) -> Option<Config> {
    let content = match fs.read(&*get_path_cstr(UKI_PREFIX, file.file_name())) {
        Ok(content) => content,
        Err(e) => {
            warn!("{e}");
            return None;
        }
    };

    let config = match UkiConfig::new(&content) {
        Ok(config) => config,
        Err(e) => {
            warn!("{e}");
            return None;
        }
    };

    Some(Config {
        title: Some(config.title),
        sort_key: Some(config.sort_key),
        version: config.version,
        efi: Some(format!("{UKI_PREFIX}\\{}", file.file_name())),
        handle: Some(*handle),
        filename: String::from(file.file_name()),
        suffix: ".efi".to_owned(),
        ..Config::default()
    })
}

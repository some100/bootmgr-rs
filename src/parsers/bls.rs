use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::warn;
use uefi::{CStr16, CString16, Handle, cstr16, fs::FileSystem, proto::media::file::FileInfo};

use crate::{
    parsers::{Config, ConfigParser},
    system::helper::{get_path_cstr, read_filtered_dir},
};

const BLS_PREFIX: &CStr16 = cstr16!("\\loader\\entries");

#[derive(Clone, Debug)]
pub struct BlsConfig {
    title: Option<String>,
    version: Option<String>,
    machine_id: Option<String>,
    sort_key: Option<String>,
    linux: Option<String>,
    initrd: Vec<String>,
    efi: Option<String>,
    options: Option<String>,
    devicetree: Option<String>,
    devicetree_overlay: Option<String>,
    architecture: Option<String>,
}

impl BlsConfig {
    fn new(content: &str) -> Self {
        let mut config = BlsConfig::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            if let Some((key, value)) = line.split_once(' ') {
                let value = value.trim().to_owned();
                match &*key.to_ascii_lowercase() {
                    "title" => config.title = Some(value),
                    "version" => config.version = Some(value),
                    "machine_id" => config.machine_id = Some(value),
                    "sort_key" => config.sort_key = Some(value),
                    "linux" => config.linux = Some(value),
                    "initrd" => config.initrd.push(value), // there may be multiple initrd entries
                    "efi" => config.efi = Some(value),
                    "options" => config.options = Some(value),
                    "devicetree" => config.devicetree = Some(value),
                    "devicetree_overlay" => config.devicetree_overlay = Some(value),
                    "architecture" => config.architecture = Some(value.to_ascii_lowercase()),
                    _ => (),
                }
            }
        }

        config
    }

    pub fn get_options(&self) -> String {
        format!(
            "{} {}",
            self.options.clone().unwrap_or_default(),
            self.initrd_options()
        )
    }

    pub fn initrd_options(&self) -> String {
        self.initrd
            .iter()
            .map(|i| format!("initrd={}", i))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl Default for BlsConfig {
    fn default() -> Self {
        Self {
            title: None,
            version: None,
            machine_id: None,
            sort_key: None,
            linux: None,
            initrd: Vec::new(),
            efi: None,
            options: None,
            devicetree: None,
            devicetree_overlay: None,
            architecture: None,
        }
    }
}

impl ConfigParser for BlsConfig {
    fn parse_configs(fs: &mut FileSystem, handle: &Handle, configs: &mut Vec<Config>) {
        let dir = read_filtered_dir(fs, BLS_PREFIX, ".conf");

        for file in dir {
            if let Some(config) = get_bls_config(&file, fs, handle) {
                configs.push(config);
            }
        }
    }
}

fn get_bls_config(file: &FileInfo, fs: &mut FileSystem, handle: &Handle) -> Option<Config> {
    let content = match fs.read_to_string(&*get_path_cstr(BLS_PREFIX, file.file_name())) {
        Ok(content) => content,
        Err(e) => {
            warn!("{e}");
            return None;
        }
    };

    let bls_config = BlsConfig::new(&content);
    let options = bls_config.get_options();

    Some(Config {
        title: bls_config.title,
        version: bls_config.version,
        machine_id: bls_config.machine_id,
        sort_key: bls_config.sort_key,
        linux: bls_config.linux,
        efi: bls_config.efi,
        options: Some(options),
        devicetree: bls_config.devicetree,
        architecture: bls_config.architecture,
        bad: check_bad(fs, file),
        handle: Some(*handle),
        filename: String::from(file.file_name()),
        suffix: ".conf".to_owned(),
        ..Config::default()
    })
}

fn check_bad(fs: &mut FileSystem, file: &FileInfo) -> bool {
    let filename = String::from(file.file_name());

    let filename = filename.trim_end_matches(".conf");
    let Some(index) = filename.rfind("+") else {
        return false; // there is no boot counting, so just say its good
    };

    let (filename, counter) = filename.split_at(index);
    let counter = &counter[1..]; // exclude +

    let counter = if let Some((left, done)) = counter.split_once('-') {
        match (left.parse::<u32>(), done.parse::<u32>()) {
            (Ok(0), _) => return true, // tries exhausted
            (Ok(left), Ok(done)) => format!("+{}-{}", left - 1, done + 1),
            _ => return false,
        }
    } else {
        match counter.parse::<u32>() {
            Ok(0) => return true, // tries exhausted somehow
            Ok(left) => format!("+{}-1", left - 1),
            _ => return false,
        }
    };

    let filename = CString16::try_from(&*format!("{filename}{counter}.conf")).unwrap();

    let _ = fs.rename(
        &*get_path_cstr(BLS_PREFIX, file.file_name()),
        &*get_path_cstr(BLS_PREFIX, &filename),
    );

    false
}

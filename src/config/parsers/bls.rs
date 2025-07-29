#![cfg(feature = "bls")]

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::{error, warn};
use uefi::{
    CStr16, CString16, Handle,
    boot::ScopedProtocol,
    cstr16,
    proto::media::{file::FileInfo, fs::SimpleFileSystem},
};

use crate::{
    BootResult,
    config::{Config, builder::ConfigBuilder, parsers::ConfigParser},
    system::{
        fs::{read_filtered_dir, read_to_string, rename},
        helper::{get_path_cstr, str_to_cstr},
    },
};

const BLS_PREFIX: &CStr16 = cstr16!("\\loader\\entries");
const BLS_SUFFIX: &str = ".conf";

struct BootCounter {
    base_name: String,
    left: u32,
    done: u32,
}

impl BootCounter {
    fn new(filename: impl Into<String>) -> Option<Self> {
        let filename = filename.into();

        let filename = filename.trim_end_matches(BLS_SUFFIX);
        let v: Vec<&str> = filename.rsplitn(2, '+').collect();

        if v.len() != 2 {
            return None;
        }

        let counter = v[0];
        let (left, done) = match counter.split_once('-') {
            Some((l, d)) => (l.parse().ok()?, d.parse().ok()?),
            None => (counter.parse().ok()?, 0),
        };

        Some(Self {
            base_name: v[1].to_owned(),
            left,
            done,
        })
    }

    fn to_filename(&self) -> BootResult<CString16> {
        let str = if self.done > 0 {
            format!("{}+{}-{}.conf", self.base_name, self.left, self.done)
        } else {
            format!("{}+{}.conf", self.base_name, self.left)
        };

        Ok(str_to_cstr(&str)?)
    }

    const fn decrement(&mut self) {
        if self.left > 0 {
            self.left -= 1;
            self.done += 1;
        }
    }

    const fn is_bad(&self) -> bool {
        self.left == 0
    }
}

/// The parser for `BootLoaderSpec` type #1 configuration files
#[derive(Default)]
pub struct BlsConfig {
    title: Option<String>,
    version: Option<String>,
    machine_id: Option<String>,
    sort_key: Option<String>,
    linux: Option<String>,
    initrd: Option<String>,
    efi: Option<String>,
    options: Option<String>,
    devicetree: Option<String>,
    devicetree_overlay: Option<String>,
    architecture: Option<String>,
}

impl BlsConfig {
    /// Creates a new [`BlsConfig`], parsing it from a BLS configuration file formatted string.
    #[must_use = "Has no effect if the result is unused"]
    pub fn new(content: &str) -> Self {
        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
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
                    "initrd" => {
                        if let Some(mut initrd) = config.initrd {
                            initrd.push(' ');
                            initrd.push_str(&value);
                            config.initrd = Some(initrd);
                        } else {
                            config.initrd = Some(value);
                        }
                    }
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

    /// Joins both options and initrd options
    #[must_use = "Has no effect if the result is unused"]
    pub fn get_options(&self) -> String {
        let mut options = String::new();
        if let Some(opts) = &self.options {
            options.push_str(opts);
        }
        self.initrd_options(&mut options);
        options
    }

    /// Obtains all specified initrd files as options for the cmdline
    pub fn initrd_options(&self, buffer: &mut String) {
        if let Some(initrd) = &self.initrd {
            for initrd in initrd.split_ascii_whitespace() {
                if !buffer.is_empty() {
                    buffer.push(' ');
                }
                buffer.push_str("initrd=");
                buffer.push_str(initrd);
            }
        }
    }
}

impl ConfigParser for BlsConfig {
    fn parse_configs(
        fs: &mut ScopedProtocol<SimpleFileSystem>,
        handle: Handle,
        configs: &mut Vec<Config>,
    ) {
        let dir = read_filtered_dir(fs, BLS_PREFIX, BLS_SUFFIX);

        for file in dir {
            match get_bls_config(&file, fs, handle) {
                Ok(Some(config)) => configs.push(config),
                Err(e) => warn!("{e}"),
                _ => (),
            }
        }
    }
}

fn get_bls_config(
    file: &FileInfo,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
) -> BootResult<Option<Config>> {
    let content = read_to_string(fs, &get_path_cstr(BLS_PREFIX, file.file_name())?)?;

    let bls_config = BlsConfig::new(&content);
    let options = bls_config.get_options();

    let Some(efi) = bls_config.linux.or(bls_config.efi) else {
        return Ok(None);
    };

    let mut config = ConfigBuilder::new(file.file_name(), BLS_SUFFIX)
        .efi(efi)
        .options(options)
        .bad(check_bad(file, fs))
        .handle(handle);

    // Pain
    if let Some(title) = bls_config.title {
        config = config.title(title);
    }

    if let Some(version) = bls_config.version {
        config = config.version(version);
    }

    if let Some(machine_id) = bls_config.machine_id {
        config = config.machine_id(machine_id);
    }

    if let Some(sort_key) = bls_config.sort_key {
        config = config.sort_key(sort_key);
    }

    if let Some(devicetree) = bls_config.devicetree {
        config = config.devicetree(devicetree);
    }

    if let Some(architecture) = bls_config.architecture {
        config = config.architecture(architecture);
    }

    Ok(Some(config.build()))
}

fn check_bad(file: &FileInfo, fs: &mut ScopedProtocol<SimpleFileSystem>) -> bool {
    let counter = BootCounter::new(file.file_name());

    if let Some(mut counter) = counter {
        if counter.is_bad() {
            return true; // tries exhausted
        }

        counter.decrement();

        let Ok(counter_name) = counter.to_filename() else {
            return false; // if we cant even convert the boot counter into a filename, just return
        };

        let Ok(src) = get_path_cstr(BLS_PREFIX, file.file_name()) else {
            return false;
        };

        let Ok(dst) = get_path_cstr(BLS_PREFIX, &counter_name) else {
            return false;
        };

        if let Err(e) = rename(fs, &src, &dst) {
            error!("{e}");
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_config() {
        let config = r"
            title Linux
            linux /vmlinuz-linux
            initrd /initramfs-linux.img
            options root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv rw
        ";
        let bls_config = BlsConfig::new(config);
        assert_eq!(bls_config.title, Some("Linux".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
        assert_eq!(bls_config.initrd, Some("/initramfs-linux.img".to_owned()));
        assert_eq!(
            bls_config.options,
            Some("root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv rw".to_owned())
        );
        assert_eq!(
            bls_config.get_options(),
            "root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv rw initrd=/initramfs-linux.img"
                .to_owned()
        );
    }

    #[test]
    fn test_multiple_initrd() {
        let config = r"
            title Linux
            linux /vmlinuz-linux
            initrd /intel-ucode.img
            initrd /initramfs-linux.img
            options root=PARTUUID=dcba4321-fe65-hg87-ji09-vutsrqponmlk ro
        ";
        let bls_config = BlsConfig::new(config);
        assert_eq!(
            bls_config.initrd,
            Some("/intel-ucode.img /initramfs-linux.img".to_owned())
        );
        assert_eq!(bls_config.get_options(), "root=PARTUUID=dcba4321-fe65-hg87-ji09-vutsrqponmlk ro initrd=/intel-ucode.img initrd=/initramfs-linux.img".to_owned());
    }

    #[test]
    fn test_comment() {
        let config = r"
            # A comment that should be ignored.
            title Linux
            linux /vmlinuz-linux
        ";
        let bls_config = BlsConfig::new(config);
        assert_eq!(bls_config.title, Some("Linux".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_duplicate() {
        let config = r"
            title Linux
            title Linux 2
            linux /vmlinuz-linux
        ";
        let bls_config = BlsConfig::new(config);
        // the last title in sequence takes priority. not based on any kind of rule or specification but a side effect of the parser implementation
        assert_eq!(bls_config.title, Some("Linux 2".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_boot_counter() {
        let filename = "somelinuxconf+3.conf";
        // we do not care about expects since its a test
        let mut ctr = BootCounter::new(filename)
            .expect("Failed to create a boot counter from filename in test");
        ctr.decrement();
        assert_eq!(
            ctr.to_filename()
                .expect("Failed to convert simple string to a filename"),
            CString16::try_from("somelinuxconf+2-1.conf")
                .expect("Failed to convert simple string to CString16")
        );
        ctr.decrement();
        assert_eq!(
            ctr.to_filename()
                .expect("Failed to convert simple string to a filename"),
            CString16::try_from("somelinuxconf+1-2.conf")
                .expect("Failed to convert simple string to CString16")
        );
        ctr.decrement();
        assert_eq!(
            ctr.to_filename()
                .expect("Failed to convert simple string to a filename"),
            CString16::try_from("somelinuxconf+0-3.conf")
                .expect("Failed to convert simple string to CString16")
        );
        assert!(ctr.is_bad());
    }
}

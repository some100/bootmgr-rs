// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! A parser for BootLoaderSpec type #1, a versionless specification for consistent boot entries.
//!
//! Example configuration:
//!
//! ```text
//! # a comment
//!
//! title Linux
//! sort_key linux
//! linux /vmlinuz-linux
//! options root=UUID=e09d636b-0cd9-4e84-8a39-84432cfc2b8e ro
//! ```

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use log::{error, warn};
use uefi::{CStr16, CString16, Handle, cstr16, proto::media::file::FileInfo};

use crate::{
    BootResult,
    config::{
        Config,
        builder::ConfigBuilder,
        parsers::{ConfigParser, Parsers},
    },
    error::BootError,
    system::{
        fs::{FsError, UefiFileSystem},
        helper::{get_path_cstr, str_to_cstr},
    },
};

/// The configuration prefix.
const BLS_PREFIX: &CStr16 = cstr16!("\\loader\\entries");

/// The configuration suffix.
const BLS_SUFFIX: &str = ".conf";

/// An implementation of the `BootLoaderSpec` boot counting feature.
///
/// A general overview of the BLS boot counting is as follows:
/// 1. The OS provides a configuration file with a boot counter annotated at the end of it (such as +3 or +3-0)
/// 2. The bootloader sees this filename and changes the boot counter to be one attempt less (+3 -> +2-1)
/// 3. If the OS is able to be booted, then it will see this boot counter on the next boot and remove the boot counter.
/// 4. Otherwise, if the boot counter is not removed, the boot loader will see this boot counter again, and rename it (+2-1 -> +1-2).
/// 5. Once the counter reaches 0 (+1-2 -> +0-3), the boot loader will mark this entry as "bad" and derank it.
///
/// This implementation will check for the boot counter, then decrement it, or if the boot counter is 0, then it will mark the entry as bad.
struct BootCounter {
    /// The base name of the configuration name (without .conf, or boot counting)
    base_name: String,

    /// The amount of tries left as in the configuration name
    left: u32,

    /// The amount of boot attempts done as in the configuration name.
    done: u32,
}

impl BootCounter {
    /// Create a new [`BootCounter`] given a filename containing a boot counter.
    ///
    /// Will return [`None`] if there is no boot counter, or the file does not contain a valid
    /// boot counter.
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

    /// Convert the current [`BootCounter`] into a filename for renaming.
    fn to_filename(&self) -> BootResult<CString16> {
        let str = if self.done > 0 {
            format!("{}+{}-{}.conf", self.base_name, self.left, self.done)
        } else {
            format!("{}+{}.conf", self.base_name, self.left)
        };

        Ok(str_to_cstr(&str)?)
    }

    /// Decrement the [`BootCounter`] if the tries were not exhausted.
    const fn decrement(&mut self) {
        if self.left > 0 {
            self.left -= 1;
            self.done += 1;
        }
    }

    /// Check if the [`BootCounter`] is bad, or if the tries left is 0.
    const fn is_bad(&self) -> bool {
        self.left == 0
    }
}

/// The parser for `BootLoaderSpec` type #1 configuration files
#[derive(Default)]
pub struct BlsConfig {
    /// The title of the configuration.
    title: Option<String>,

    /// The version of the configuration.
    version: Option<String>,

    /// The machine-id of the configuration.
    machine_id: Option<String>,

    /// The sort-key of the configuration.
    sort_key: Option<String>,

    /// The linux path of the configuration.
    linux: Option<String>,

    /// The initrds of the configuration.
    initrd: Option<String>,

    /// The efi path of the configuration.
    efi: Option<String>,

    /// The options of the configuration.
    options: Option<String>,

    /// The devicetree path of the configuration.
    devicetree: Option<String>,

    /// The devicetree overlay path of the configuration.
    devicetree_overlay: Option<String>,

    /// The architecture of the configuration.
    architecture: Option<String>,
}

impl BlsConfig {
    /// Creates a new [`BlsConfig`], parsing it from a BLS configuration file formatted string.
    ///
    /// The amount of bytes to parse as UTF-8 should be provided if required, otherwise it will be determined by
    /// the byte slice length.
    ///
    /// If there are multiple key-value pairs of the same type, then the latest one will be used.
    /// This is not for any reason in particular, it is more of a side effect of the way the parser is implemented.
    #[must_use = "Has no effect if the result is unused"]
    pub fn new(content: &[u8], bytes: Option<usize>) -> Self {
        let mut config = Self::default();
        let slice = &content[0..bytes.unwrap_or(content.len()).min(content.len())];

        if let Ok(content) = str::from_utf8(slice) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                config.assign_to_field(line);
            }
        }

        config
    }

    /// Assign a field to the [`BlsConfig`] given a line containing the key and value.
    fn assign_to_field(&mut self, line: &str) {
        if let Some((key, value)) = line.split_once(' ') {
            let value = value.trim().to_owned();
            match &*key.to_ascii_lowercase() {
                "title" => self.title = Some(value),
                "version" => self.version = Some(value),
                "machine_id" => self.machine_id = Some(value),
                "sort_key" => self.sort_key = Some(value),
                "linux" => self.linux = Some(value),
                "initrd" => {
                    if let Some(initrd) = &mut self.initrd {
                        initrd.push(' ');
                        initrd.push_str(&value);
                    } else {
                        self.initrd = Some(value);
                    }
                }
                "efi" => self.efi = Some(value),
                "options" => self.options = Some(value),
                "devicetree" => self.devicetree = Some(value),
                "devicetree_overlay" => self.devicetree_overlay = Some(value),
                "architecture" => self.architecture = Some(value.to_ascii_lowercase()),
                _ => warn!("[BLS PARSER]: Found unrecognized key {key} with value {value}"),
            }
        }
    }

    /// Joins both options and initrd options
    #[must_use = "Has no effect if the result is unused"]
    fn get_options(&self) -> String {
        let mut options = String::new();
        if let Some(opts) = &self.options {
            options.push_str(opts);
        }
        self.initrd_options(&mut options);
        options
    }

    /// Obtains all specified initrd files as options for the cmdline
    fn initrd_options(&self, buffer: &mut String) {
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
    fn parse_configs(fs: &mut UefiFileSystem, handle: Handle, configs: &mut Vec<Config>) {
        let dir = fs.read_filtered_dir(BLS_PREFIX, BLS_SUFFIX);

        for file in dir {
            match get_bls_config(&file, fs, handle) {
                Ok(Some(config)) => configs.push(config),
                Err(e) => warn!("{e}"),
                _ => (),
            }
        }
    }
}

/// Parse a BLS file given the [`FileInfo`], a `SimpleFileSystem` protocol, and a handle to that protocol.
fn get_bls_config(
    file: &FileInfo,
    fs: &mut UefiFileSystem,
    handle: Handle,
) -> BootResult<Option<Config>> {
    let mut buf = [0; 4096]; // preallocated buffer big enough for most config files
    let path = get_path_cstr(BLS_PREFIX, file.file_name())?;
    let read_result = fs.read_into(&path, &mut buf);

    // if the file was too big for the buffer, it will use read instead, which allocates on the heap
    // the size of the file.
    let (bytes, buf) = match read_result {
        Ok(bytes) => (bytes, &buf[..]),
        Err(FsError::BufTooSmall(bytes)) => (bytes, &fs.read(&path)?[..]),
        Err(e) => return Err(BootError::FsError(e)),
    };

    let bls_config = BlsConfig::new(buf, Some(bytes));
    let options = bls_config.get_options();

    let Some(efi_path) = bls_config.linux.or(bls_config.efi) else {
        return Ok(None);
    };

    let config = ConfigBuilder::new(file.file_name(), BLS_SUFFIX)
        .efi_path(efi_path)
        .options(options)
        .set_bad(check_bad(file, fs))
        .fs_handle(handle)
        .origin(Parsers::Bls)
        .assign_if_some(bls_config.title, ConfigBuilder::title)
        .assign_if_some(bls_config.version, ConfigBuilder::version)
        .assign_if_some(bls_config.machine_id, ConfigBuilder::machine_id)
        .assign_if_some(bls_config.sort_key, ConfigBuilder::sort_key)
        .assign_if_some(bls_config.devicetree, ConfigBuilder::devicetree_path)
        .assign_if_some(bls_config.architecture, ConfigBuilder::architecture);

    Ok(Some(config.build()))
}

/// Check if a certain config is bad given the [`FileInfo`] and a `SimpleFileSystem` protocol.
fn check_bad(file: &FileInfo, fs: &mut UefiFileSystem) -> bool {
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

        if let Err(e) = fs.rename(&src, &dst) {
            error!("{e}");
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn test_basic_config() {
        let config = b"
            title Linux
            linux /vmlinuz-linux
            initrd /initramfs-linux.img
            options root=PARTUUID=1234abcd-56ef-78gh-90ij-klmnopqrstuv rw
        ";
        let bls_config = BlsConfig::new(config, None);
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
        let config = b"
            title Linux
            linux /vmlinuz-linux
            initrd /intel-ucode.img
            initrd /initramfs-linux.img
            options root=PARTUUID=dcba4321-fe65-hg87-ji09-vutsrqponmlk ro
        ";
        let bls_config = BlsConfig::new(config, None);
        assert_eq!(
            bls_config.initrd,
            Some("/intel-ucode.img /initramfs-linux.img".to_owned())
        );
        assert_eq!(bls_config.get_options(), "root=PARTUUID=dcba4321-fe65-hg87-ji09-vutsrqponmlk ro initrd=/intel-ucode.img initrd=/initramfs-linux.img".to_owned());
    }

    #[test]
    fn test_comment() {
        let config = b"
            # A comment that should be ignored.
            title Linux
            linux /vmlinuz-linux
        ";
        let bls_config = BlsConfig::new(config, None);
        assert_eq!(bls_config.title, Some("Linux".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_duplicate() {
        let config = b"
            title Linux
            title Linux 2
            linux /vmlinuz-linux
        ";
        let bls_config = BlsConfig::new(config, None);
        // the last title in sequence takes priority. not based on any kind of rule or specification but a side effect of the parser implementation
        assert_eq!(bls_config.title, Some("Linux 2".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_invalid_keys() {
        let config = b"
            title Linux
            invalid invalid
            someother invalid
        ";
        let bls_config = BlsConfig::new(config, None);
        assert_eq!(bls_config.title, Some("Linux".to_owned())); // valid keys should still be parsed
    }

    #[test]
    fn test_boot_counter() {
        let filename = "somelinuxconf+3.conf";

        // if this panics, it indicates a failure in the boot counter parser.
        let mut ctr = BootCounter::new(filename)
            .expect("Failed to create a boot counter from valid filename in test");
        ctr.decrement();
        assert_eq!(
            ctr.to_filename().ok(),
            CString16::try_from("somelinuxconf+2-1.conf").ok()
        );
        ctr.decrement();
        assert_eq!(
            ctr.to_filename().ok(),
            CString16::try_from("somelinuxconf+1-2.conf").ok()
        );
        ctr.decrement();
        assert_eq!(
            ctr.to_filename().ok(),
            CString16::try_from("somelinuxconf+0-3.conf").ok()
        );
        assert!(ctr.is_bad());
    }

    proptest! {
        #[test]
        fn doesnt_panic(x in any::<Vec<u8>>(), y in any::<usize>()) {
            let _ = BlsConfig::new(&x, Some(y));
        }

        #[test]
        fn sets_title(x in any::<String>()) {
            let x = x.trim();
            let title = format!("title {x}");
            let config = BlsConfig::new(title.as_bytes(), None);
            if !x.is_empty() {
                prop_assert_eq!(config.title, Some(x.to_owned()));
            }
        }
    }
}

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
    error::BootError,
    system::{
        fs::{FsError, read, read_filtered_dir, read_into, rename},
        helper::{get_path_cstr, str_to_cstr},
    },
};

/// The configuration prefix.
const BLS_PREFIX: &CStr16 = cstr16!("\\loader\\entries");

/// The configuration suffix.
const BLS_SUFFIX: &str = ".conf";

/// An implementation of the `BootLoaderSpec` boot counting feature.
pub struct BootCounter {
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
    pub fn new(filename: impl Into<String>) -> Option<Self> {
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
        let slice = &content[0..bytes.unwrap_or(content.len())];

        if let Ok(content) = str::from_utf8(slice) {
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

/// Parse a BLS file given the [`FileInfo`], a [`SimpleFileSystem`] protocol, and a handle to that protocol.
fn get_bls_config(
    file: &FileInfo,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    handle: Handle,
) -> BootResult<Option<Config>> {
    let mut buf = [0; 4096]; // preallocated buffer big enough for most config files
    let path = get_path_cstr(BLS_PREFIX, file.file_name())?;
    let read_result = read_into(fs, &path, &mut buf);

    // if the file was too big for the buffer, it will use read instead, which allocates on the heap
    // the size of the file.
    let (bytes, buf) = match read_result {
        Ok(bytes) => (bytes, &buf[..]),
        Err(BootError::FsError(FsError::BufTooSmall(bytes))) => (bytes, &read(fs, &path)?[..]),
        Err(e) => return Err(e),
    };

    let bls_config = BlsConfig::new(buf, Some(bytes));
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

/// Check if a certain config is bad given the [`FileInfo`] and a [`SimpleFileSystem`] protocol.
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
        "
        .as_bytes();
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
        let config = r"
            title Linux
            linux /vmlinuz-linux
            initrd /intel-ucode.img
            initrd /initramfs-linux.img
            options root=PARTUUID=dcba4321-fe65-hg87-ji09-vutsrqponmlk ro
        "
        .as_bytes();
        let bls_config = BlsConfig::new(config, None);
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
        "
        .as_bytes();
        let bls_config = BlsConfig::new(config, None);
        assert_eq!(bls_config.title, Some("Linux".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_duplicate() {
        let config = r"
            title Linux
            title Linux 2
            linux /vmlinuz-linux
        "
        .as_bytes();
        let bls_config = BlsConfig::new(config, None);
        // the last title in sequence takes priority. not based on any kind of rule or specification but a side effect of the parser implementation
        assert_eq!(bls_config.title, Some("Linux 2".to_owned()));
        assert_eq!(bls_config.linux, Some("/vmlinuz-linux".to_owned()));
    }

    #[test]
    fn test_invalid_keys() {
        let config = r"
            title Linux
            invalid invalid
            someother invalid
        "
        .as_bytes();
        let bls_config = BlsConfig::new(config, None);
        assert_eq!(bls_config.title, Some("Linux".to_owned())); // valid keys should still be parsed
    }

    #[test]
    fn test_boot_counter() {
        let filename = "somelinuxconf+3.conf";
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
}

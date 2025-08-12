//! Filesystem helper functions for other modules.
//!
//! These mostly wrap around the UEFI [`SimpleFileSystem`] protocol to make an interface that's slightly more
//! intuitive and more in line with the Rust standard library.
//!
//! These filesystem helpers are guaranteed to support FAT filesystems. This is mandated by the UEFI specification. However, UEFI firmwares
//! are not forced to support solely FAT32. It is perfectly possible and even simple to support non FAT filesystems, using EFI filesystem
//! drivers.
//!
//! Examples of such drivers implementing [`SimpleFileSystem`] include those found in [efifs](https://efi.akeo.ie), which are built
//! off of GRUB's drivers, as well as [Ext4Pkg](https://github.com/acidanthera/audk/tree/master/Ext4Pkg). This means that filesystems
//! ranging from Ext4 to Btrfs and ZFS can be supported due to the pluggable nature of UEFI drivers. Note however that drivers must be
//! signed before loading if you are using Secure Boot (or enrolled with MOK if you're using a custom Shim build).
//!
//! These drivers can be installed in `\EFI\BOOT\drivers` of the same EFI partition as `bootmgr-rs`, and `bootmgr-rs` will automatically
//! load those drivers for usage in scanning for `Config`s. Alternatively, if the firmware supports those filesystems in the first place,
//! then `bootmgr-rs` will already be able to scan those drivers. You also have to explicitly enable those drivers in `BootConfig`.
//!
//! This module also provides filesystem-related testing functions, like [`UefiFileSystem::exists`].

use alloc::{borrow::ToOwned, boxed::Box, string::String, vec, vec::Vec};
use log::error;
use thiserror::Error;
use uefi::{
    CStr16, CString16, Char16, Handle, Status,
    boot::{self, ScopedProtocol},
    fs::{CHARACTER_DENY_LIST, COMMON_SKIP_DIRS, UefiDirectoryIter},
    guid,
    proto::media::{
        file::{
            Directory, File, FileAttribute, FileInfo, FileMode, FileSystemVolumeLabel, RegularFile,
        },
        fs::SimpleFileSystem,
        partition::{GptPartitionType, PartitionInfo},
    },
};

use crate::{BootResult, system::helper::str_to_cstr};

/// The size of one gigabyte in bytes. This is the default value if a file is too big to be read.
///
/// This is also a reasonable maximum size for files that may be read.
pub(crate) const ONE_GIGABYTE: usize = 1024 * 1024 * 1024;

/// The partition GUID of an `XBOOTLDR` partition.
const XBOOTLDR_PARTITION: uefi::Guid = guid!("bc13c2ff-59e6-4262-a352-b275fd6f7172");

/// An error that may result from performing filesystem operations
#[derive(Error, Debug)]
pub enum FsError {
    /// The provided buffer was too small.
    #[error("Buffer too small (require {0} bytes)")]
    BufTooSmall(usize),

    /// The content could not be written to the file.
    #[error("Could not write to file: returned status {status} ({bytes} bytes written)")]
    WriteErr {
        /// The error status that was returned from the attempted write.
        status: Status,

        /// The amount of bytes that were written.
        bytes: usize,
    },

    /// A file could not be opened.
    #[error("Failed to open file")]
    OpenErr(Status),

    /// A file could not be read.
    #[error("Failed to read file")]
    ReadErr(Status),

    /// A file could not be deleted.
    #[error("Failed to delete file")]
    DeleteErr(Status),

    /// A file could not be flushed.
    #[error("Failed to flush file")]
    FlushErr(Status),

    /// A seek operation was attempted to be made on a deleted file
    #[error("Could not set position of a deleted file")]
    SeekErr,

    /// Failed to get a volume label on a partition.
    #[error("Could not get volume label of a partition")]
    VolumeLabelErr,
}

/// A rust-ier wrapper around [`SimpleFileSystem`].
///
/// This is similar to [`uefi::fs::FileSystem`], with different design decisions.
pub struct UefiFileSystem(ScopedProtocol<SimpleFileSystem>);

impl UefiFileSystem {
    /// Create a new [`UefiFileSystem`].
    #[must_use = "Has no effect if the result is unused"]
    pub const fn new(fs: ScopedProtocol<SimpleFileSystem>) -> Self {
        Self(fs)
    }

    /// Create a new [`UefiFileSystem`] from a handle that supports [`SimpleFileSystem`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the handle does not actually support [`SimpleFileSystem`].
    pub fn from_handle(handle: Handle) -> BootResult<Self> {
        let fs = boot::open_protocol_exclusive(handle)?;
        Ok(Self(fs))
    }

    /// Create a new [`UefiFileSystem`] from the same filesystem as the boot manager.
    ///
    /// This is mainly used when the boot manager wants to read from a file on the same filesystem as itself (for example,
    /// the `BootConfig` file).
    ///
    /// # Errors
    ///
    /// May return an `Error` if the boot image's filesystem does not support [`SimpleFileSystem`] for some reason.
    pub fn from_image_fs() -> BootResult<Self> {
        let fs = boot::get_image_file_system(boot::image_handle())?;
        Ok(Self(fs))
    }

    /// Gets the volume label from a [`SimpleFileSystem`]
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume could not be opened, or the volume does not support [`FileSystemVolumeLabel`]
    pub fn get_volume_label(&mut self) -> Result<CString16, FsError> {
        let mut root = self
            .0
            .open_volume()
            .map_err(|x| FsError::OpenErr(x.status()))?;
        let info = root
            .get_boxed_info::<FileSystemVolumeLabel>()
            .map_err(|_| FsError::VolumeLabelErr)?;
        Ok(info.volume_label().to_owned())
    }

    /// Checks if a file exists from a [`Handle`] to a partition.
    ///
    /// It makes no distinction between whether a file could not be verified to exist or a file that really
    /// does not exist. Both will return `false`. This means that if the volume could not be opened, it will return
    /// `false` as the file cannot be verified to exist.
    pub fn exists(&mut self, path: &CStr16) -> bool {
        let Ok(mut root) = self.0.open_volume() else {
            return false;
        };

        root.open(path, FileMode::Read, FileAttribute::empty())
            .is_ok()
    }

    /// Checks if a file exists from a handle to a partition with an [`&str`] path.
    ///
    /// This is simply a helper function that converts an [`&str`] to a [`CString16`] so that it
    /// may be used with the [`Self::exists`] function.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the path could not be converted into a [`CString16`]
    pub fn exists_str(&mut self, path: &str) -> BootResult<bool> {
        Ok(self.exists(&str_to_cstr(path)?))
    }

    /// Returns a [`UefiDirectoryIter`] of files in the path from a handle to a partition.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the path does not exist.
    pub fn read_dir(&mut self, path: &CStr16) -> Result<UefiDirectoryIter, FsError> {
        Ok(UefiDirectoryIter::new(self.get_directory(path)?))
    }

    /// Returns an iterator of [`FileInfo`]s that filter out non-matching files.
    ///
    /// This applies several filters to ensure that the file matches as expected. "." and ".."
    /// are displayed in directory lists, so they are filtered out. Then, the filename's suffix is
    /// compared to the provided extension and filtered out if they don't match. Finally, the
    /// file is filtered if it is empty.
    pub fn read_filtered_dir(
        &mut self,
        path: &CStr16,
        ext: &'static str,
    ) -> impl Iterator<Item = Box<FileInfo>> + use<> {
        // use<> needed due to rust 2024
        self.read_dir(path)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|x| !COMMON_SKIP_DIRS.contains(&x.file_name())) // excludes "." and ".."
            .filter(move |x| {
                String::from(x.file_name())
                    .to_ascii_lowercase()
                    .ends_with(ext)
            })
            .filter(|x| x.file_size() > 0)
    }

    /// Attempts to read as much as possible of a file into a byte buffer.
    /// On success it will also return the amount of bytes read.
    ///
    /// You may want to use [`core::str::from_utf8`] to convert the content into an &str.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file,
    /// the file could not be read for any reason, or the buffer was too small. If the buffer was too small,
    /// the amount of bytes required is returned.
    pub fn read_into(&mut self, path: &CStr16, buf: &mut [u8]) -> Result<usize, FsError> {
        let mut file = self.get_regular_file(path)?;

        let info = file
            .get_boxed_info::<FileInfo>()
            .map_err(|e| FsError::ReadErr(e.status()))?;

        let size = usize::try_from(info.file_size()).unwrap_or(ONE_GIGABYTE);

        let read = file.read(buf).map_err(|e| FsError::ReadErr(e.status()))?;
        if read != size {
            return Err(FsError::BufTooSmall(size));
        }

        Ok(read)
    }

    /// Reads the entire content of a file into a [`Vec<u8>`].
    ///
    /// You may want to use [`core::str::from_utf8`] to convert the content into an &str.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file, or
    /// the file could not be read for any reason.
    pub fn read(&mut self, path: &CStr16) -> Result<Vec<u8>, FsError> {
        let mut file = self.get_regular_file(path)?;

        let info = file
            .get_boxed_info::<FileInfo>()
            .map_err(|e| FsError::ReadErr(e.status()))?;

        let size = usize::try_from(info.file_size()).unwrap_or(ONE_GIGABYTE);

        let mut buf = vec![0; size];
        file.read(&mut buf)
            .map_err(|e| FsError::ReadErr(e.status()))?;

        Ok(buf)
    }

    /// Renames a file into another file.
    ///
    /// This essentially copies a file into another file, then deletes the original file. This implements buffered
    /// reading and writing, with a fixed size of 64 KiB. This is small enough to fit the majority of cases this is
    /// used (like for BLS boot counting).
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, any of the two paths don't point to a valid file,
    /// the source file could not be read, or the source file could not be deleted.
    pub fn rename(&mut self, src: &CStr16, dst: &CStr16) -> Result<(), FsError> {
        let _ = self.delete(dst);
        let _ = self.create(dst); // this way if dst exists or not, it will be created anyways

        let mut src = self.get_mut_file(src)?;
        let mut dst = self.get_mut_file(dst)?;

        let mut chunk = vec![0; 64 * 1024]; // 64 kib buffer

        let src_info = src
            .get_boxed_info::<FileInfo>()
            .map_err(|e| FsError::ReadErr(e.status()))?;
        let mut remaining = src_info.file_size();

        while remaining > 0 {
            let bytes = src
                .read(&mut chunk)
                .map_err(|e| FsError::ReadErr(e.status()))?;

            if bytes == 0 {
                return Err(FsError::ReadErr(Status::ABORTED));
            }

            dst.write(&chunk[..bytes]).map_err(|e| FsError::WriteErr {
                status: e.status(),
                bytes: *e.data(),
            })?;

            remaining -= u64::try_from(bytes).unwrap_or(u64::MAX);
        }
        dst.flush().map_err(|e| FsError::FlushErr(e.status()))?;

        src.delete().map_err(|e| FsError::DeleteErr(e.status()))?;

        Ok(())
    }

    /// Creates an empty file.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume could not be opened.
    pub fn create(&mut self, path: &CStr16) -> Result<(), FsError> {
        let mut root = self
            .0
            .open_volume()
            .map_err(|x| FsError::OpenErr(x.status()))?;
        let f = root
            .open(path, FileMode::CreateReadWrite, FileAttribute::empty())
            .map_err(|e| FsError::OpenErr(e.status()))?;

        if let Some(mut f) = f.into_regular_file() {
            let buf = [0; 0];
            let _ = f.write(&buf);
        }
        Ok(())
    }

    /// Writes a byte slice into a file.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, or the file does not exist.
    pub fn write(&mut self, path: &CStr16, buffer: &[u8]) -> Result<(), FsError> {
        let mut file = self.get_mut_file(path)?;

        file.write(buffer).map_err(|e| FsError::WriteErr {
            status: e.status(),
            bytes: *e.data(),
        })?;

        Ok(())
    }

    /// Appends a byte slice onto a file.
    ///
    /// This is similar to using [`Self::write`] only that instead of replacing the content of a file from the beginning,
    /// it adds new content onto the end of a file.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, or the file does not exist.
    pub fn append(&mut self, path: &CStr16, buffer: &[u8]) -> BootResult<()> {
        let mut file = self.get_mut_file(path)?;
        file.set_position(RegularFile::END_OF_FILE)
            .map_err(|_| FsError::SeekErr)?;

        file.write(buffer).map_err(|e| FsError::WriteErr {
            status: e.status(),
            bytes: *e.data(),
        })?;

        Ok(())
    }

    /// Deletes a file.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file,
    /// or the file could not be deleted.
    pub fn delete(&mut self, path: &CStr16) -> Result<(), FsError> {
        let file = self.get_mut_file(path)?;
        file.delete().map_err(|e| FsError::DeleteErr(e.status()))?;

        Ok(())
    }

    /// Gets a handle to a [`RegularFile`] in the filesystem.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, or the path does not point to a file.
    fn get_regular_file(&mut self, path: &CStr16) -> Result<RegularFile, FsError> {
        let mut root = self
            .0
            .open_volume()
            .map_err(|e| FsError::OpenErr(e.status()))?;
        root.open(path, FileMode::Read, FileAttribute::empty())
            .map_err(|e| FsError::OpenErr(e.status()))?
            .into_regular_file()
            .ok_or(FsError::OpenErr(Status::INVALID_PARAMETER))
    }

    /// Gets a handle to a [`RegularFile`] that is writable in the filesystem.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, or the path does not point to a file.
    fn get_mut_file(&mut self, path: &CStr16) -> Result<RegularFile, FsError> {
        let mut root = self
            .0
            .open_volume()
            .map_err(|e| FsError::OpenErr(e.status()))?;
        root.open(path, FileMode::ReadWrite, FileAttribute::empty())
            .map_err(|e| FsError::OpenErr(e.status()))?
            .into_regular_file()
            .ok_or(FsError::OpenErr(Status::INVALID_PARAMETER))
    }

    /// Gets a handle to a [`Directory`] in the filesystem.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the volume couldn't be opened, or the path does not point to a folder.
    fn get_directory(&mut self, path: &CStr16) -> Result<Directory, FsError> {
        let mut root = self
            .0
            .open_volume()
            .map_err(|e| FsError::OpenErr(e.status()))?;
        root.open(path, FileMode::ReadWrite, FileAttribute::empty())
            .map_err(|e| FsError::OpenErr(e.status()))?
            .into_directory()
            .ok_or(FsError::OpenErr(Status::INVALID_PARAMETER))
    }
}

/// Checks if a partition is an EFI System Partition or an XBOOTLDR partition.
///
/// This will only work if the handle supports [`PartitionInfo`], else it will return
/// [`true`] for every partition.
#[must_use = "Has no effect if the result is unused"]
pub(crate) fn is_target_partition(handle: Handle) -> bool {
    // for filesystems that support partitioninfo, filter partitions by guid
    if let Ok(info) = boot::open_protocol_exclusive::<PartitionInfo>(handle) {
        let Some(entry) = info.gpt_partition_entry() else {
            return false;
        };
        let guid = entry.partition_type_guid;
        if guid != GptPartitionType::EFI_SYSTEM_PARTITION
            && guid != GptPartitionType(XBOOTLDR_PARTITION)
        {
            return false;
        }
    }
    true
}

/// Checks if an [`&str`] path is valid.
///
/// If a path contains any one of the characters: `"`, `*`, `/`, `:`, `<`, `>`, `?`, and `|`,
/// this will return false. It will also return false if the path consists only of `..` or `.`.
#[must_use = "Has no effect if the result is unused"]
pub(crate) fn check_path_valid(path: &str) -> bool {
    path.chars()
        .all(|x| Char16::try_from(x).is_ok_and(|x| !CHARACTER_DENY_LIST.contains(&x) || x == '\\'))
        && path != ".."
        && path != "."
}

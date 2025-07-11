#![allow(clippy::cast_possible_truncation)]
//! Filesystem helper functions for other modules

use alloc::{borrow::ToOwned, boxed::Box, string::String, vec, vec::Vec};
use log::error;
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

use crate::{error::BootError, system::helper::str_to_cstr};

const XBOOTLDR_PARTITION: uefi::Guid = guid!("bc13c2ff-59e6-4262-a352-b275fd6f7172");

/// Gets the volume label from a `SimpleFileSystem`
///
/// # Errors
///
/// May return an `Error` if the volume could not be opened, or the volume does not support [`FileSystemVolumeLabel`]
pub fn get_volume_label(fs: &mut ScopedProtocol<SimpleFileSystem>) -> uefi::Result<CString16> {
    let mut root = fs.open_volume()?;
    let info = root.get_boxed_info::<FileSystemVolumeLabel>()?;
    Ok(info.volume_label().to_owned())
}

/// Checks if a partition is an EFI System Partition or an XBOOTLDR partition.
///
/// This will only work if the handle supports [`PartitionInfo`], else it will return
/// [`true`] for every partition.
#[must_use]
pub fn is_target_partition(handle: &Handle) -> bool {
    // for filesystems that support partitioninfo, filter partitions by guid
    if let Ok(info) = boot::open_protocol_exclusive::<PartitionInfo>(*handle) {
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

/// Checks if a file exists from a [`Handle`] to a partition.
///
/// # Errors
///
/// May return an `Error` if the handle does not support `SimpleFileSystem`, or the path
/// contains invalid UCS-2 characters, or there was a nul character found in the input
pub fn check_file_exists(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> Result<bool, BootError> {
    let mut root = fs.open_volume()?;

    match root.open(path, FileMode::Read, FileAttribute::empty()) {
        Ok(_) => Ok(true),
        Err(e) if e.status() == Status::NOT_FOUND => Ok(false),
        Err(e) => Err(BootError::Uefi(e)),
    }
}

/// Checks if a file exists from a handle to a partition with an [`&str`] path.
///
/// # Errors
///
/// May return an `Error` if the path contains invalid UCS-2 characters, or there was a nul character found in the input
pub fn check_file_exists_str(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &str,
) -> Result<bool, BootError> {
    check_file_exists(fs, &str_to_cstr(path))
}

/// Checks if an [`&str`] path is valid.
///
/// If a path contains any one of the characters: `"`, `*`, `/`, `:`, `<`, `>`, `?`, and `|`,
/// this will return false.
#[must_use]
pub fn check_path_valid(path: &str) -> bool {
    path.chars()
        .all(|x| Char16::try_from(x).is_ok_and(|x| !CHARACTER_DENY_LIST.contains(&x) || x == '\\'))
}

/// Deletes a file.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file,
/// or the file could not be deleted.
pub fn delete(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> uefi::Result<()> {
    let file = get_regular_file(fs, path)?;
    file.delete()?;

    Ok(())
}

/// Gets a handle to a [`RegularFile`] in the filesystem.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the path does not point to a folder.
pub fn get_regular_file(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> uefi::Result<RegularFile> {
    let mut root = fs.open_volume()?;
    root.open(path, FileMode::Read, FileAttribute::empty())?
        .into_regular_file()
        .ok_or::<uefi::Error>(Status::INVALID_PARAMETER.into())
}

/// Gets a handle to a [`Directory`] in the filesystem.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the path does not point to a folder.
pub fn get_directory(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> uefi::Result<Directory> {
    let mut root = fs.open_volume()?;
    root.open(path, FileMode::Read, FileAttribute::empty())?
        .into_directory()
        .ok_or::<uefi::Error>(Status::INVALID_PARAMETER.into())
}

/// Returns a [`UefiDirectoryIter`] of files in the path from a handle to a partition.
///
/// # Errors
///
/// May return an `Error` if the path does not exist.
pub fn read_dir(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> uefi::Result<UefiDirectoryIter> {
    Ok(UefiDirectoryIter::new(get_directory(fs, path)?))
}

/// Returns an iterator of [`FileInfo`]s that filter out non-matching files.
///
/// This applies several filters to ensure that the file matches as expected. "." and ".."
/// are displayed in directory lists, so they are filtered out. Then, the filename's suffix is
/// compared to the provided extension and filtered out if they don't match. Finally, the
/// file is filtered if it is empty.
pub fn read_filtered_dir(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
    ext: &'static str,
) -> impl Iterator<Item = Box<FileInfo>> + use<> {
    // no clue why this is needed, something about rust 2024 edition
    read_dir(fs, path)
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

/// Reads the entire content of a file into a [`Vec<u8>`].
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file, or
/// the file could not be read for any reason.
pub fn read(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> uefi::Result<Vec<u8>> {
    let mut file = get_regular_file(fs, path)?;

    let info = file.get_boxed_info::<FileInfo>()?;

    let mut buf = vec![0; info.file_size() as usize];
    let read = file.read(&mut buf)?;
    if read != info.file_size() as usize {
        error!("{}/{} bytes read", read, info.file_size());
    }

    Ok(buf)
}

/// Reads the entire content of a file into a [`String`].
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file, or
/// the file could not be read for any reason, or the file contains non UTF-8 characters.
pub fn read_to_string(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> uefi::Result<String> {
    Ok(String::from_utf8(read(fs, path)?).map_err(|_| Status::INVALID_PARAMETER)?)
}

/// Renames a file into another file.
///
/// This essentially copies a file into another file, then deletes the original file. This copies the entire
/// content of the source file into memory, so it should not be used for very large files.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, any of the two paths don't point to a valid file,
/// the source file could not be read, or the source file could not be deleted.
pub fn rename(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    src: &CStr16,
    dst: &CStr16,
) -> uefi::Result<()> {
    let _ = delete(fs, dst);
    let src_data = read(fs, src)?;

    let src = get_regular_file(fs, src)?;
    let mut dst = get_regular_file(fs, dst)?;

    if let Err(e) = dst.write(&src_data) {
        error!("{}/{} bytes were written", e.data(), src_data.len());
    }

    src.delete()?;

    Ok(())
}

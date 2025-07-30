//! Filesystem helper functions for other modules.
//!
//! These mostly wrap around the UEFI [`SimpleFileSystem`] protocol to make an interface that's slightly more
//! intuitive and more in line with the Rust standard library. This is clear from functions like [`read_to_string`].
//! This module also provides filesystem-related testing functions, like [`check_file_exists`].

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

use crate::{BootResult, error::BootError, system::helper::str_to_cstr};

/// The partition GUID of an `XBOOTLDR` partition.
const XBOOTLDR_PARTITION: uefi::Guid = guid!("bc13c2ff-59e6-4262-a352-b275fd6f7172");

/// Gets the volume label from a `SimpleFileSystem`
///
/// # Errors
///
/// May return an `Error` if the volume could not be opened, or the volume does not support [`FileSystemVolumeLabel`]
pub fn get_volume_label(fs: &mut ScopedProtocol<SimpleFileSystem>) -> BootResult<CString16> {
    let mut root = fs.open_volume()?;
    let info = root.get_boxed_info::<FileSystemVolumeLabel>()?;
    Ok(info.volume_label().to_owned())
}

/// Checks if a partition is an EFI System Partition or an XBOOTLDR partition.
///
/// This will only work if the handle supports [`PartitionInfo`], else it will return
/// [`true`] for every partition.
#[must_use = "Has no effect if the result is unused"]
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
/// It makes no distinction between whether a file could not be verified to exist or a file that really
/// does not exist. Both will return `false`. This means that if the volume could not be opened, it will return
/// `false` as the file cannot be verified to exist.
pub fn check_file_exists(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> bool {
    let Ok(mut root) = fs.open_volume() else {
        return false;
    };

    root.open(path, FileMode::Read, FileAttribute::empty())
        .is_ok()
}

/// Checks if a file exists from a handle to a partition with an [`&str`] path.
///
/// This is simply a helper function that converts an [`&str`] to a [`CString16`] so that it
/// may be used with the [`check_file_exists`] function.
///
/// # Errors
///
/// May return an `Error` if the path could not be converted into a [`CString16`]
pub fn check_file_exists_str(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &str,
) -> BootResult<bool> {
    Ok(check_file_exists(fs, &str_to_cstr(path)?))
}

/// Checks if an [`&str`] path is valid.
///
/// If a path contains any one of the characters: `"`, `*`, `/`, `:`, `<`, `>`, `?`, and `|`,
/// this will return false. It will also return false if the path consists only of `..` or `.`.
#[must_use = "Has no effect if the result is unused"]
pub fn check_path_valid(path: &str) -> bool {
    path.chars()
        .all(|x| Char16::try_from(x).is_ok_and(|x| !CHARACTER_DENY_LIST.contains(&x) || x == '\\'))
        && path != ".."
        && path != "."
}

/// Deletes a file.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, the path does not point to a valid file,
/// or the file could not be deleted.
pub fn delete(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> BootResult<()> {
    let file = get_mut_file(fs, path)?;
    file.delete()?;

    Ok(())
}

/// Gets a handle to a [`RegularFile`] in the filesystem.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the path does not point to a file.
pub fn get_regular_file(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> BootResult<RegularFile> {
    let mut root = fs.open_volume()?;
    root.open(path, FileMode::Read, FileAttribute::empty())?
        .into_regular_file()
        .ok_or_else(|| BootError::Uefi(Status::INVALID_PARAMETER.into()))
}

/// Gets a handle to a [`RegularFile`] that is writable in the filesystem.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the path does not point to a file.
pub fn get_mut_file(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> BootResult<RegularFile> {
    let mut root = fs.open_volume()?;
    root.open(path, FileMode::ReadWrite, FileAttribute::empty())?
        .into_regular_file()
        .ok_or_else(|| BootError::Uefi(Status::INVALID_PARAMETER.into()))
}

/// Gets a handle to a [`Directory`] in the filesystem.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the path does not point to a folder.
pub fn get_directory(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> BootResult<Directory> {
    let mut root = fs.open_volume()?;
    root.open(path, FileMode::Read, FileAttribute::empty())?
        .into_directory()
        .ok_or_else(|| BootError::Uefi(Status::INVALID_PARAMETER.into()))
}

/// Returns a [`UefiDirectoryIter`] of files in the path from a handle to a partition.
///
/// # Errors
///
/// May return an `Error` if the path does not exist.
pub fn read_dir(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
) -> BootResult<UefiDirectoryIter> {
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
pub fn read(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> BootResult<Vec<u8>> {
    let mut file = get_regular_file(fs, path)?;

    let info = file.get_boxed_info::<FileInfo>()?;

    // the max file size of a FAT32 file system is less than usize::MAX.
    // so this should generally be safe for reading from local filesystems
    let size = match usize::try_from(info.file_size()) {
        Ok(size) => size,
        _ => usize::MAX,
    };

    let mut buf = vec![0; size];
    let read = file.read(&mut buf)?;
    if read != size {
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
) -> BootResult<String> {
    String::from_utf8(read(fs, path)?)
        .map_err(|_| BootError::Uefi(Status::INVALID_PARAMETER.into()))
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
) -> BootResult<()> {
    let _ = delete(fs, dst);
    let _ = create(fs, dst); // this way if dst exists or not, it will be created anyways
    let src_data = read(fs, src)?;

    let src = get_mut_file(fs, src)?;
    let mut dst = get_mut_file(fs, dst)?;

    if let Err(e) = dst.write(&src_data) {
        error!("{}/{} bytes were written", e.data(), src_data.len());
    }

    src.delete()?;

    Ok(())
}

/// Creates an empty file.
///
/// # Errors
///
/// May return an `Error` if the volume could not be opened.
pub fn create(fs: &mut ScopedProtocol<SimpleFileSystem>, path: &CStr16) -> BootResult<()> {
    let mut root = fs.open_volume()?;
    let f = root.open(path, FileMode::CreateReadWrite, FileAttribute::empty())?;
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
pub fn write(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
    buffer: &[u8],
) -> BootResult<()> {
    let mut file = get_mut_file(fs, path)?;

    if let Err(e) = file.write(buffer) {
        error!(
            "Failed to write: {}. Only {} bytes were written",
            e.status(),
            e.data()
        );
    }

    Ok(())
}

/// Appends a byte slice onto a file.
///
/// This is similar to using [`write()`] only that instead of replacing the content of a file from the beginning,
/// it adds new content onto the end of a file.
///
/// # Errors
///
/// May return an `Error` if the volume couldn't be opened, or the file does not exist.
pub fn append(
    fs: &mut ScopedProtocol<SimpleFileSystem>,
    path: &CStr16,
    buffer: &[u8],
) -> BootResult<()> {
    let mut file = get_mut_file(fs, path)?;
    file.set_position(RegularFile::END_OF_FILE)?;

    if let Err(e) = file.write(buffer) {
        error!("Only {} bytes were written", e.data());
    }

    Ok(())
}

// SPDX-FileCopyrightText: 2025 some100 <ootinnyoo@outlook.com>
// SPDX-License-Identifier: MIT

//! Handles devicetree installations and fixups.
//!
//! This will install a `Devicetree` into the UEFI configuration table, and may optionally
//! apply fixups if the firmware supports it via the [`DevicetreeFixup`] protocol.
//!
//! This is mostly based off of systemd-boot's implementation.
//!
//! # Safety
//!
//! This module uses unsafe in 4 places currently. This is obviously not preferable since unsafe blocks can destroy
//! the guarantees that safe Rust carries, however the places where this module uses unsafe are completely safe.
//!
//! 1. The "size" passed to `from_raw_parts_mut` is guaranteed to be the size of the allocated memory. In addition, since
//!    we have just allocated that memory, it is guaranteed to be non-null and safe to use.
//! 2. Unsafe is required to install the configuration table, because of two conditions that are upheld by the program.
//!    The first is that the data must not be freed, which is ensured by `DevicetreeGuard` leaking the memory after
//!    installation. The second is that the data must not be modified, which is similarly ensured by `DevicetreeGuard`
//!    consuming the inner devicetree.
//! 3. Unsafe is required to free the allocated memory from [`boot::allocate_pool`]. It is called only when dropped, so
//!    there cannot possibly be any remaining references to the inner pointer after it goes out of scope.

use core::ffi::c_void;
use core::ptr::NonNull;

use thiserror::Error;
use uefi::boot::ScopedProtocol;
use uefi::{guid, prelude::*};

use crate::BootResult;
use crate::error::BootError;
use crate::system::fs::UefiFileSystem;
use crate::system::helper::{get_arch, locate_protocol, normalize_path, str_to_cstr};
use crate::system::protos::DevicetreeFixup;

/// GUID for the configuration table for devicetree blobs.
const DTB_CONF_TABLE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

/// Flag indicating that fixups should be applied to the devicetree blob.
const EFI_DT_APPLY_FIXUPS: u32 = 0x0000_0001;

/// Flag indicating that memory should be reserved.
const EFI_DT_RESERVE_MEMORY: u32 = 0x0000_0002;

/// An `Error` that may result from loading a devicetree.
#[derive(Error, Debug)]
pub enum DevicetreeError {
    /// The Devicetree Guard was already consumed.
    #[error("The DevicetreeGuard was already consumed")]
    DevicetreeGuardConsumed,
}

struct Devicetree<'a> {
    /// The size of the devicetree blob.
    size: usize,

    /// A [`NonNull`] pointer to the devicetree blob.
    ptr: NonNull<u8>,

    /// The devicetree blob as a slice.
    slice: &'a [u8],
}

/// A RAII guard for [`Devicetree`] that leaks upon installation, in order to
/// safely install the devicetree blob to the configuration table.
#[must_use = "Will drop the inner Devicetree if immediately dropped"]
struct DevicetreeGuard<'a>(Option<Devicetree<'a>>);

impl Devicetree<'_> {
    /// Get a new [`Devicetree`] given a byte slice of a devicetree blob, and optionally its size which is
    /// determined from the content length if excluded.
    ///
    /// If the size provided is smaller than the content length, then the content length will be used.
    /// This ensures that both enough memory can be allocated and the slice can be constructed safely.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the memory pool could not be allocated.
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        let size = size.unwrap_or(content.len()).min(content.len());

        // ptr must be an allocation of type ACPI_RECLAIM, because dtb data must be ACPI_RECLAIM
        let mut ptr = boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, size)?;

        // SAFETY: size should be at most the size of the actual content. ptr is valid for at least
        // the length of size, as we have allocated that many bytes, so this is safe.
        let slice = unsafe { core::slice::from_raw_parts_mut(ptr.as_mut(), size) };

        slice.copy_from_slice(content);

        Ok(Self { size, ptr, slice })
    }

    /// Apply fixups to the devicetree blob with the [`DevicetreeFixup`] protocol.
    ///
    /// We simply pass the devicetree blob as well as size as is to the firmware's fixup
    /// protocol. From this point, it is up to the firmware to interpret and fixup the
    /// devicetree.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the firmware fixup method failed.
    fn fixup(&mut self, fixup: &mut ScopedProtocol<DevicetreeFixup>) -> BootResult<()> {
        Ok(fixup
            .fixup(
                self.ptr,
                &mut self.size,
                EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY,
            )
            .to_result()?)
    }

    /// Install the devicetree blob into the configuration table.
    ///
    /// # Errors
    ///
    /// May return an `Error` if we ran out of memory somehow.
    fn install(&self) -> BootResult<()> {
        // SAFETY: the ptr is not modified or freed afterwards, especially when using DevicetreeGuard, so this is
        // safe.
        unsafe {
            Ok(boot::install_configuration_table(
                &DTB_CONF_TABLE,
                self.ptr.as_ptr() as *const c_void,
            )?)
        }
    }
}

impl Drop for Devicetree<'_> {
    fn drop(&mut self) {
        // SAFETY: if the devicetree is out of scope, it will not be used again, so this is safe
        // this will only error if the ptr is invalid (such as if it wasn't allocated by allocate_pool)
        unsafe {
            let _ = boot::free_pool(self.ptr);
        }
    }
}

impl DevicetreeGuard<'_> {
    /// Get a new [`DevicetreeGuard`] given a byte slice to a devicetree blob and optionally its size.
    /// This delegates to the inner [`Devicetree`] constructor.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the inner [`Devicetree`] constructor fails.
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        Ok(Self(Some(Devicetree::new(content, size)?)))
    }

    /// Apply fixups to the devicetree blob. This delegates to the inner [`Devicetree`].
    ///
    /// # Errors
    ///
    /// May return an `Error` if the inner [`Devicetree`] fails fixup.
    fn fixup(&mut self, fixup: &mut ScopedProtocol<DevicetreeFixup>) -> BootResult<()> {
        if let Some(devicetree) = &mut self.0 {
            devicetree.fixup(fixup)?;
        }
        Ok(())
    }

    /// Install the devicetree into the configuration table. This delegates to the inner [`Devicetree`],
    /// but also leaks the pointer so that it may safely stay in the configuration table.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the inner devicetree could not be installed.
    fn install(&mut self) -> BootResult<()> {
        let devicetree = self.0.take();
        if let Some(devicetree) = devicetree {
            devicetree.install()?;
            core::mem::forget(devicetree); // pointer must not be freed or modified after installation
        }
        Ok(())
    }

    /// Get the size of the devicetree blob.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the [`DevicetreeGuard`] was already consumed.
    fn size(&self) -> Result<usize, DevicetreeError> {
        Ok(self
            .0
            .as_ref()
            .ok_or(DevicetreeError::DevicetreeGuardConsumed)?
            .size)
    }

    /// Get the devicetree blob as a byte slice.
    ///
    /// # Errors
    ///
    /// May return an `Error` if the [`DevicetreeGuard`] was already consumed.
    fn slice(&self) -> Result<&[u8], DevicetreeError> {
        Ok(self
            .0
            .as_ref()
            .ok_or(DevicetreeError::DevicetreeGuardConsumed)?
            .slice)
    }
}

impl Drop for DevicetreeGuard<'_> {
    fn drop(&mut self) {
        self.0.take();
    }
}

/// Lets the firmware apply fixups to the provided devicetree.
///
/// This essentially attempts to open the [`DevicetreeFixup`] protocol on the system if it exists, then running its fixup method
/// in order to apply firmware fixups to a DTB blob.
///
/// This also handles resizing the buffer, in case it is too small for the fixup method. This is possible because it may return
/// an error of status `BUFFER_TOO_SMALL`, with the bytes needed as the payload.
///
/// # Errors
///
/// May return an `Error` if the firmware does not support [`DevicetreeFixup`], or the devicetree could not be converted into a slice,
/// or the devicetree failed to fixup after resizing the buffer.
fn fixup_devicetree(devicetree: &mut DevicetreeGuard) -> BootResult<()> {
    let mut fixup = locate_protocol::<DevicetreeFixup>()?;

    let slice = devicetree.slice()?.to_vec();

    if let Err(BootError::Uefi(e)) = devicetree.fixup(&mut fixup)
        && e.status() == Status::BUFFER_TOO_SMALL
    {
        *devicetree = DevicetreeGuard::new(&slice, Some(devicetree.size()?))?;
        devicetree.fixup(&mut fixup)?;
    }

    Ok(())
}

/// Installs a given devicetree into the FDT DTB table.
///
/// Optionally, if available it calls the firmware's devicetree fixup protocol,
/// so that the firmware may apply fixups to the provided devicetree.
///
/// This will do absolutely nothing if the system is not an ARM system.
///
/// # Errors
///
/// May return an `Error` if the devicetree path is not valid, or memory allocation fails. If there is failure
/// anywhere after memory is allocated, then the data is freed.
pub(super) fn install_devicetree(devicetree: &str, fs: &mut UefiFileSystem) -> BootResult<()> {
    if matches!(
        get_arch().as_deref().map(alloc::string::String::as_str),
        Some("arm" | "aa64") // these are the only archs requiring devicetree supported by both uefi and rust
    ) {
        let path = str_to_cstr(&normalize_path(devicetree))?;
        let f = fs.read(&path)?;

        let mut devicetree = DevicetreeGuard::new(&f, None)?;

        fixup_devicetree(&mut devicetree)?;

        devicetree.install()?;
    }

    Ok(())
}

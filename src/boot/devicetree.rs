//! Handles devicetree installations and fixups.
//!
//! This will install a `Devicetree` into the UEFI configuration table, and may optionally
//! apply fixups if the firmware supports it via the [`DevicetreeFixup`] protocol.
//!
//! This is mostly based off of systemd-boot's implementation.

use core::ffi::c_void;
use core::ptr::{NonNull, copy_nonoverlapping};

use thiserror::Error;
use uefi::boot::ScopedProtocol;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::{guid, prelude::*};

use crate::BootResult;
use crate::error::BootError;
use crate::system::fs::read;
use crate::system::helper::{normalize_path, str_to_cstr};
use crate::system::protos::DevicetreeFixup;

const DTB_CONF_TABLE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
const DTB_FIXUP_TABLE: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
const EFI_DT_APPLY_FIXUPS: u32 = 0x0000_0001;
const EFI_DT_RESERVE_MEMORY: u32 = 0x0000_0002;

/// An `Error` that may result from loading a devicetree.
#[derive(Error, Debug)]
pub enum DevicetreeError {
    /// The Devicetree Guard was already consumed.
    #[error("The DevicetreeGuard was already consumed")]
    DevicetreeGuardConsumed,
}

struct Devicetree {
    size: usize,
    ptr: NonNull<u8>,
}

#[must_use = "Will drop the inner Devicetree if immediately dropped"]
struct DevicetreeGuard {
    devicetree: Option<Devicetree>,
}

impl Devicetree {
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        let size = size.unwrap_or(content.len());
        let ptr = boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, size)?;
        unsafe {
            // SAFETY: ptr is exactly the same length as size, so this is safe
            copy_nonoverlapping(content.as_ptr(), ptr.as_ptr(), content.len());
        }
        Ok(Self { size, ptr })
    }

    fn fixup(&mut self, fixup: &mut ScopedProtocol<DevicetreeFixup>) -> BootResult<()> {
        unsafe {
            // SAFETY: self.ptr is guaranteed NonNull
            Ok(fixup
                .fixup(
                    self.ptr.as_ptr().cast::<c_void>(),
                    &mut self.size,
                    EFI_DT_APPLY_FIXUPS | EFI_DT_RESERVE_MEMORY,
                )
                .to_result()?)
        }
    }

    fn install(&self) -> BootResult<()> {
        unsafe {
            Ok(boot::install_configuration_table(
                &DTB_CONF_TABLE,
                self.ptr.as_ptr() as *const c_void,
            )?)
        }
    }
}

impl Drop for Devicetree {
    fn drop(&mut self) {
        unsafe {
            // SAFETY: if the devicetree is out of scope, it will not be used again, so this is safe
            // this will only error if the ptr is invalid (such as if it wasn't allocated by allocate_pool)
            let _ = boot::free_pool(self.ptr);
        }
    }
}

impl DevicetreeGuard {
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        Ok(Self {
            devicetree: Some(Devicetree::new(content, size)?),
        })
    }

    fn fixup(&mut self, fixup: &mut ScopedProtocol<DevicetreeFixup>) -> BootResult<()> {
        if let Some(devicetree) = &mut self.devicetree {
            devicetree.fixup(fixup)?;
        }
        Ok(())
    }

    fn install(&mut self) -> BootResult<()> {
        let devicetree = self.devicetree.take();
        if let Some(devicetree) = devicetree {
            devicetree.install()?;
            core::mem::forget(devicetree); // pointer must not be freed or modified after installation
        }
        Ok(())
    }

    fn size(&self) -> Result<usize, DevicetreeError> {
        Ok(self
            .devicetree
            .as_ref()
            .ok_or(DevicetreeError::DevicetreeGuardConsumed)?
            .size)
    }

    fn ptr(&self) -> Result<NonNull<u8>, DevicetreeError> {
        Ok(self
            .devicetree
            .as_ref()
            .ok_or(DevicetreeError::DevicetreeGuardConsumed)?
            .ptr)
    }

    fn as_slice<'a>(&self) -> Result<&'a [u8], DevicetreeError> {
        unsafe {
            Ok(core::slice::from_raw_parts(
                self.ptr()?.as_ptr(),
                self.size()?,
            ))
        }
    }
}

impl Drop for DevicetreeGuard {
    fn drop(&mut self) {
        let devicetree = self.devicetree.take();
        if let Some(devicetree) = devicetree {
            drop(devicetree);
        }
    }
}

// Lets the firmware apply fixups to the provided devicetree.
fn fixup_devicetree(devicetree: &mut DevicetreeGuard) -> BootResult<()> {
    let Ok(fixup) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&DTB_FIXUP_TABLE))
    else {
        return Ok(()); // do nothing if the firmware does not offer fixups
    };

    let Some(fixup) = fixup.first() else {
        return Err(BootError::Uefi(uefi::Status::NOT_FOUND.into())); // this shouldnt happen in any case
    };

    let mut fixup = boot::open_protocol_exclusive::<DevicetreeFixup>(*fixup)?;

    let devtree_as_slice = devicetree.as_slice()?;

    if let Err(BootError::Uefi(e)) = devicetree.fixup(&mut fixup)
        && e.status() == Status::BUFFER_TOO_SMALL
    {
        *devicetree = DevicetreeGuard::new(devtree_as_slice, Some(devicetree.size()?))?;
        devicetree.fixup(&mut fixup)?;
    }

    Ok(())
}

/// Installs a given devicetree into the FDT DTB table.
///
/// Optionally, if available it calls the firmware's devicetree fixup protocol,
/// so that the firmware may apply fixups to the provided devicetree.
///
/// # Errors
///
/// May return an `Error` if the devicetree path is not valid, the handle does not
/// support [`SimpleFileSystem`], or memory allocation fails. If there is failure
/// anywhere after memory is allocated, then the data is freed.
pub fn install_devicetree(
    devicetree: &str,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
) -> BootResult<()> {
    let path = str_to_cstr(&normalize_path(devicetree))?;
    let f = read(fs, &path)?;

    let mut devicetree = DevicetreeGuard::new(&f, None)?;

    fixup_devicetree(&mut devicetree)?;

    devicetree.install()?;

    Ok(())
}

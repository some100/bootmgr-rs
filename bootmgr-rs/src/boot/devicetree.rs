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
use crate::system::helper::{get_arch, normalize_path, str_to_cstr};
use crate::system::protos::DevicetreeFixup;

/// GUID for the configuration table for devicetree blobs.
const DTB_CONF_TABLE: uefi::Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

/// GUID for the configuration table for devicetree fixups.
const DTB_FIXUP_TABLE: uefi::Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");

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

struct Devicetree {
    /// The size of the devicetree blob.
    size: usize,

    /// A [`NonNull`] pointer to the devicetree blob.
    ptr: NonNull<u8>,

    /// The devicetree blob as a slice.
    slice: &'static [u8],
}

/// A RAII guard for [`Devicetree`] that leaks upon installation, in order to
/// safely install the devicetree blob to the configuration table.
#[must_use = "Will drop the inner Devicetree if immediately dropped"]
struct DevicetreeGuard {
    /// The inner [`Devicetree`].
    devicetree: Option<Devicetree>,
}

impl Devicetree {
    /// Get a new [`Devicetree`] given a byte slice of a devicetree blob, and optionally its size which is
    /// determined from the content length if excluded.
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        let size = size.unwrap_or(content.len());

        // ptr must be an allocation of type ACPI_RECLAIM, because dtb data must be ACPI_RECLAIM
        let ptr = boot::allocate_pool(boot::MemoryType::ACPI_RECLAIM, size)?;
        unsafe {
            // SAFETY: ptr is exactly the same length as size, so this is safe
            copy_nonoverlapping(content.as_ptr(), ptr.as_ptr(), content.len());
        }
        let slice = unsafe { core::slice::from_raw_parts(ptr.as_ptr(), size) }; // store the slice in the struct
        Ok(Self { size, ptr, slice })
    }

    /// Apply fixups to the devicetree blob with the [`DevicetreeFixup`] protocol.
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
    fn install(&self) -> BootResult<()> {
        unsafe {
            // SAFETY: the ptr is not modified or freed afterwards, especially when using DevicetreeGuard
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
    /// Get a new [`DevicetreeGuard`] given a byte slice to a devicetree blob and optionally its size.
    /// This delegates to the inner [`Devicetree`] constructor.
    fn new(content: &[u8], size: Option<usize>) -> BootResult<Self> {
        Ok(Self {
            devicetree: Some(Devicetree::new(content, size)?),
        })
    }

    /// Apply fixups to the devicetree blob. This delegates to the inner [`Devicetree`].
    fn fixup(&mut self, fixup: &mut ScopedProtocol<DevicetreeFixup>) -> BootResult<()> {
        if let Some(devicetree) = &mut self.devicetree {
            devicetree.fixup(fixup)?;
        }
        Ok(())
    }

    /// Install the devicetree into the configuration table. This delegates to the inner [`Devicetree`],
    /// but also leaks the pointer so that it may safely stay in the configuration table.
    fn install(&mut self) -> BootResult<()> {
        let devicetree = self.devicetree.take();
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
            .devicetree
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
            .devicetree
            .as_ref()
            .ok_or(DevicetreeError::DevicetreeGuardConsumed)?
            .slice)
    }
}

impl Drop for DevicetreeGuard {
    fn drop(&mut self) {
        self.devicetree.take();
    }
}

/// Lets the firmware apply fixups to the provided devicetree.
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
    let Ok(fixup) = boot::locate_handle_buffer(boot::SearchType::ByProtocol(&DTB_FIXUP_TABLE))
    else {
        return Ok(()); // do nothing if the firmware does not offer fixups
    };

    let Some(fixup) = fixup.first() else {
        return Err(BootError::Uefi(uefi::Status::NOT_FOUND.into())); // this shouldnt happen in any case
    };

    let mut fixup = boot::open_protocol_exclusive::<DevicetreeFixup>(*fixup)?;

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
/// May return an `Error` if the devicetree path is not valid, the handle does not
/// support [`SimpleFileSystem`], or memory allocation fails. If there is failure
/// anywhere after memory is allocated, then the data is freed.
pub fn install_devicetree(
    devicetree: &str,
    fs: &mut ScopedProtocol<SimpleFileSystem>,
) -> BootResult<()> {
    if matches!(
        get_arch().as_deref().map(alloc::string::String::as_str),
        Some("arm" | "aa64")
    ) {
        let path = str_to_cstr(&normalize_path(devicetree))?;
        let f = read(fs, &path)?;

        let mut devicetree = DevicetreeGuard::new(&f, None)?;

        fixup_devicetree(&mut devicetree)?;

        devicetree.install()?;
    }

    Ok(())
}
